use std::{
    fmt::Display,
    io::{Cursor, Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use binrw::{binread, BinReaderExt};
use destiny_pkg::{package::UEntryHeader, TagHash, TagHash64};
use eframe::egui::load::SizedTexture;
use eframe::egui::{vec2, TextureId};
use eframe::egui_wgpu::RenderState;
use eframe::{
    egui::{self, CollapsingHeader, RichText},
    epaint::Color32,
    wgpu,
};
use itertools::Itertools;
use log::error;
use nohash_hasher::{IntMap, IntSet};
use poll_promise::Promise;
use std::fmt::Write;

use crate::{gui::texture::Texture, util::u32_from_endian};
use crate::{
    packages::package_manager,
    references::REFERENCE_MAP,
    scanner::{ScanResult, TagCache},
    tagtypes::TagType,
    text::StringCache,
};

use super::{
    common::{open_tag_in_default_application, tag_context},
    View,
};

pub struct TagView {
    cache: Arc<TagCache>,
    string_cache: Arc<StringCache>,
    string_hashes: Vec<(u64, u32)>,
    raw_strings: Vec<(u64, String)>,
    arrays: Vec<(u64, TagArray)>,

    textures: IntMap<TagHash, (Texture, TextureId)>,

    tag: TagHash,
    tag64: Option<TagHash64>,
    tag_entry: UEntryHeader,
    tag_type: TagType,

    scan: ExtendedScanResult,
    tag_traversal: Option<Promise<String>>,
    traversal_depth_limit: usize,
    traversal_show_strings: bool,
    start_time: Instant,

    render_state: RenderState,
}

impl TagView {
    pub fn create(
        cache: Arc<TagCache>,
        string_cache: Arc<StringCache>,
        tag: TagHash,
        render_state: RenderState,
    ) -> Option<TagView> {
        let tag_data = package_manager().read_tag(tag).ok()?;
        let mut array_offsets = vec![];
        let mut raw_string_offsets = vec![];
        let mut string_hashes = vec![];

        let endian = package_manager().version.endian();
        for (i, b) in tag_data.chunks_exact(4).enumerate() {
            let v: [u8; 4] = b.try_into().unwrap();
            let value = u32_from_endian(endian, v);
            let offset = i as u64 * 4;

            if matches!(value, 0x80809fb8 | 0x80800184) {
                array_offsets.push(offset + 4);
            }

            if value == 0x80800065 {
                raw_string_offsets.push(offset);
            }

            if string_cache.contains_key(&value) {
                string_hashes.push((offset, value));
            }
        }

        let raw_strings = raw_string_offsets
            .into_iter()
            .flat_map(|o| read_raw_string_blob(&tag_data, o))
            .collect_vec();

        let mut arrays: Vec<(u64, TagArray)> = if package_manager().version.is_d1() {
            array_offsets
                .into_iter()
                .filter_map(|o| {
                    let mut c = Cursor::new(&tag_data);
                    c.seek(SeekFrom::Start(o)).ok()?;
                    Some((
                        o,
                        TagArray {
                            count: c.read_be::<u32>().ok()? as _,
                            tagtype: c.read_be::<u32>().ok()?,
                            references: vec![],
                        },
                    ))
                })
                .collect_vec()
        } else {
            array_offsets
                .into_iter()
                .filter_map(|o| {
                    let mut c = Cursor::new(&tag_data);
                    c.seek(SeekFrom::Start(o)).ok()?;
                    Some((o, c.read_le().ok()?))
                })
                .collect_vec()
        };

        let mut cur = Cursor::new(&tag_data);
        loop {
            let offset = cur.stream_position().unwrap();
            let Ok((value1, value2)) = cur.read_le::<(u64, u64)>() else {
                break;
            };

            let possibly_count = value1;
            let possibly_array_offset = (offset + 8).saturating_add(value2);

            if let Some((_, array)) = arrays.iter_mut().find(|(offset, arr)| {
                *offset == possibly_array_offset && arr.count == possibly_count
            }) {
                array.references.push(offset);
            }

            cur.seek(SeekFrom::Current(-8)).unwrap();
        }

        let tag64 = package_manager()
            .hash64_table
            .iter()
            .find(|(_, e)| e.hash32 == tag)
            .map(|(&h64, _)| TagHash64(h64));

        let tag_entry = package_manager().get_entry(tag)?;
        let scan = ExtendedScanResult::from_scanresult(cache.hashes.get(&tag).cloned()?);

        let mut textures: IntMap<TagHash, (Texture, TextureId)> = Default::default();
        for hash in &scan.file_hashes {
            if let Some(t) = &hash.entry {
                let tagtype = TagType::from_type_subtype(t.file_type, t.file_subtype);
                if tagtype.is_texture() {
                    match Texture::load(&render_state, hash.hash.hash32()) {
                        Ok(t) => {
                            let egui_handle =
                                render_state.renderer.write().register_native_texture(
                                    &render_state.device,
                                    &t.view,
                                    wgpu::FilterMode::Linear,
                                );

                            textures.insert(hash.hash.hash32(), (t, egui_handle));
                        }
                        Err(e) => {
                            error!("Failed to load texture {}: {e}", hash.hash);
                        }
                    }
                }
            }
        }

        Some(Self {
            arrays,
            string_hashes,
            tag,
            tag64,
            tag_type: TagType::from_type_subtype(tag_entry.file_type, tag_entry.file_subtype),
            tag_entry,
            textures,

            scan,
            cache,
            traversal_depth_limit: 16,
            tag_traversal: None,
            traversal_show_strings: false,
            string_cache,
            raw_strings,
            start_time: Instant::now(),
            render_state,
        })
    }

    /// Replaces this view with another tag
    pub fn open_tag(&mut self, tag: TagHash) {
        if let Some(tv) = Self::create(
            self.cache.clone(),
            self.string_cache.clone(),
            tag,
            self.render_state.clone(),
        ) {
            *self = tv;
        } else {
            error!("Could not open new tag view for {tag} (tag not found in cache)");
        }
    }
}

impl View for TagView {
    fn view(&mut self, ctx: &eframe::egui::Context, ui: &mut eframe::egui::Ui) {
        let mut open_new_tag = None;

        ui.heading(format_tag_entry(self.tag, Some(&self.tag_entry)))
            .context_menu(|ui| tag_context(ui, self.tag, self.tag64));

        ui.label(
            RichText::new(format!(
                "Package {}",
                package_manager()
                    .package_paths
                    .get(&self.tag.pkg_id())
                    .map(|p| Path::new(p).file_name().unwrap_or_default())
                    .unwrap_or_default()
                    .to_string_lossy()
            ))
            .weak(),
        );

        ui.horizontal(|ui| {
            if ui.button("Open tag data in external application").clicked() {
                open_tag_in_default_application(self.tag);
            }

            if TagHash(self.tag_entry.reference).is_pkg_file()
                && ui
                    .button("Open referenced in external application")
                    .clicked()
            {
                open_tag_in_default_application(self.tag_entry.reference.into());
            }
        });

        ui.separator();
        egui::SidePanel::left("tv_left_panel")
            .resizable(true)
            .min_width(256.0)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CollapsingHeader::new(
                        egui::RichText::new("Files referencing this tag").strong(),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.scan.references.is_empty() {
                            ui.label(RichText::new("No incoming references found").italics());
                        } else {
                            for (tag, entry) in &self.scan.references {
                                let fancy_tag = format_tag_entry(*tag, Some(entry));
                                let response = ui.add_enabled(
                                    *tag != self.tag,
                                    egui::SelectableLabel::new(false, fancy_tag),
                                );

                                if response
                                    .context_menu(|ui| tag_context(ui, *tag, None))
                                    .clicked()
                                {
                                    open_new_tag = Some(*tag);
                                }
                            }
                        }
                    });

                    CollapsingHeader::new(
                        egui::RichText::new("Tag references in this file").strong(),
                    )
                    .default_open(true)
                    .show(ui, |ui| {
                        if self.scan.file_hashes.is_empty() {
                            ui.label(RichText::new("No outgoing references found").italics());
                        } else {
                            for tag in &self.scan.file_hashes {
                                let mut is_texture = false;
                                let offset_label = if tag.offset == u64::MAX {
                                    "TagHeader reference".to_string()
                                } else {
                                    format!("0x{:X}", tag.offset)
                                };

                                let tag_label = if let Some(entry) = &tag.entry {
                                    let tagtype = TagType::from_type_subtype(
                                        entry.file_type,
                                        entry.file_subtype,
                                    );
                                    is_texture = tagtype.is_texture();

                                    let fancy_tag =
                                        format_tag_entry(tag.hash.hash32(), Some(entry));

                                    egui::RichText::new(format!("{fancy_tag} @ {offset_label}"))
                                        .color(tagtype.display_color())
                                } else {
                                    egui::RichText::new(format!(
                                        "{} (pkg entry not found) @ {offset_label}",
                                        tag.hash
                                    ))
                                    .color(Color32::LIGHT_RED)
                                };

                                // TODO(cohae): Highlight/jump to tag in hex viewer
                                let response = ui.add_enabled(
                                    tag.hash.hash32() != self.tag,
                                    egui::SelectableLabel::new(false, tag_label),
                                );

                                ctx.style_mut(|s| s.interaction.show_tooltips_only_when_still = false);
                                if response
                                    .context_menu(|ui| {
                                        tag_context(
                                            ui,
                                            tag.hash.hash32(),
                                            match tag.hash {
                                                ExtendedTagHash::Hash32(_) => None,
                                                ExtendedTagHash::Hash64(t) => Some(t),
                                            },
                                        )
                                    })
                                    .on_hover_ui(|ui| {
                                        if is_texture {
                                            if let Some((tex, egui_tex)) =
                                                self.textures.get(&tag.hash.hash32())
                                            {
                                                let max_height = ui.available_height() * 0.90;

                                                let tex_size = if ui.input(|i| i.modifiers
                                                    .ctrl) {
                                                    vec2(max_height * tex.aspect_ratio, max_height)
                                                } else {
                                                    ui.label("ℹ Hold ctrl to enlarge");
                                                    vec2(256. * tex.aspect_ratio, 256.)
                                                };

                                                ui.image(SizedTexture::new(*egui_tex, tex_size));

                                                ui.label(format!("{}x{}x{} {:?}", tex.width, tex.height, tex.depth, tex.format));
                                            } else {
                                                ui.colored_label(Color32::RED, "⚠ Texture not found, check log for more information");
                                            }
                                        }
                                    })
                                    .clicked()
                                {
                                    open_new_tag = Some(tag.hash.hash32());
                                }
                            }
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if !self.scan.successful {
                ui.heading(RichText::new("⚠ Tag data failed to read").color(Color32::YELLOW));
            }

            if self.tag_type.is_tag() {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.tag_traversal
                                .as_ref()
                                .map(|v| v.poll().is_ready())
                                .unwrap_or(true),
                            egui::Button::new("Traverse children"),
                        )
                        .clicked()
                    {
                        let tag = self.tag;
                        let cache = self.cache.clone();
                        let depth_limit = self.traversal_depth_limit;
                        let show_strings = self.traversal_show_strings;
                        self.tag_traversal =
                            Some(Promise::spawn_thread("traverse tags", move || {
                                traverse_tags(tag, depth_limit, cache, show_strings)
                            }));
                    }

                    if ui.button("Copy traversal").clicked() {
                        if let Some(traversal) = self.tag_traversal.as_ref() {
                            if let Some(result) = traversal.ready() {
                                ui.output_mut(|o| o.copied_text = result.clone());
                            }
                        }
                    }

                    ui.add(
                        egui::DragValue::new(&mut self.traversal_depth_limit).clamp_range(1..=256),
                    );
                    ui.label("Max depth");

                    ui.checkbox(
                        &mut self.traversal_show_strings,
                        "Find strings (currently only shows raw strings)",
                    );
                });

                if let Some(traversal) = self.tag_traversal.as_ref() {
                    if let Some(result) = traversal.ready() {
                        egui::ScrollArea::both()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.style_mut().wrap = Some(false);
                                ui.label(RichText::new(result).monospace());
                            });
                    } else {
                        ui.spinner();
                        ui.label("Traversing tags");
                    }
                }
            } else {
                ui.label(RichText::new("Traversal not available for non-8080 tags").italics());
            }
        });

        if !self.string_hashes.is_empty() || !self.raw_strings.is_empty() || !self.arrays.is_empty()
        {
            egui::SidePanel::right("tv_right_panel")
                .resizable(true)
                .min_width(320.0)
                .show_inside(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(egui::RichText::new("Arrays").strong());
                        ui.group(|ui| {
                            if self.arrays.is_empty() {
                                ui.label(RichText::new("No arrays found").italics());
                            } else {
                                for (offset, array) in &self.arrays {
                                    let ref_label = REFERENCE_MAP
                                        .read()
                                        .get(&array.tagtype)
                                        .map(|s| format!("{s} ({:08X})", array.tagtype.to_be()))
                                        .unwrap_or_else(|| {
                                            format!("{:08X}", array.tagtype.to_be())
                                        });

                                    ui.selectable_label(
                                        false,
                                        format!(
                                            "type={} count={} @ 0x{:X}",
                                            ref_label, array.count, offset
                                        ),
                                    )
                                    .on_hover_text({
                                        if array.references.is_empty() {
                                            "⚠ Array is not referenced!".to_string()
                                        } else {
                                            format!(
                                                "Referenced at {}",
                                                array
                                                    .references
                                                    .iter()
                                                    .map(|o| format!("0x{o:X}"))
                                                    .join(", ")
                                            )
                                        }
                                    })
                                    .clicked();
                                }
                            }
                        });

                        ui.label(egui::RichText::new("String Hashes").strong());
                        ui.group(|ui| {
                            if self.string_hashes.is_empty() {
                                ui.label(RichText::new("No strings found").italics());
                            } else {
                                for (offset, hash) in &self.string_hashes {
                                    if let Some(strings) = self.string_cache.get(hash) {
                                        if strings.len() > 1 {
                                            ui.selectable_label(
                                                false,
                                                format!(
                                                    "'{}' ({} collisions) {:08x} @ 0x{:X}",
                                                    strings[(self.start_time.elapsed().as_secs()
                                                        as usize)
                                                        % strings.len()],
                                                    strings.len(),
                                                    hash,
                                                    offset
                                                ),
                                            )
                                            .on_hover_text(strings.join("\n"))
                                            .clicked();
                                        } else {
                                            ui.selectable_label(
                                                false,
                                                format!(
                                                    "'{}' {:08x} @ 0x{:X}",
                                                    strings[0], hash, offset
                                                ),
                                            )
                                            .clicked();
                                        }
                                    }
                                }
                            }
                        });

                        ui.label(egui::RichText::new("Raw strings (65008080)").strong());
                        ui.group(|ui| {
                            if self.raw_strings.is_empty() {
                                ui.label(RichText::new("No raw strings found").italics());
                            } else {
                                for (offset, string) in &self.raw_strings {
                                    ui.selectable_label(
                                        false,
                                        format!("'{}' @ 0x{:X}", string, offset),
                                    )
                                    .context_menu(|ui| {
                                        if ui.selectable_label(false, "Copy text").clicked() {
                                            ui.output_mut(|o| o.copied_text = string.clone());
                                            ui.close_menu();
                                        }
                                    })
                                    .clicked();
                                }
                            }
                        });
                    });
                });
        }

        ctx.request_repaint_after(Duration::from_secs(1));

        if let Some(new_tag) = open_new_tag {
            self.open_tag(new_tag)
        }
    }
}

pub enum ExtendedTagHash {
    Hash32(TagHash),
    Hash64(TagHash64),
}

impl ExtendedTagHash {
    pub fn hash32(&self) -> TagHash {
        match self {
            ExtendedTagHash::Hash32(h) => *h,
            ExtendedTagHash::Hash64(h) => package_manager()
                .hash64_table
                .get(&h.0)
                .map(|v| v.hash32)
                .unwrap_or(TagHash::NONE),
        }
    }
}

impl Display for ExtendedTagHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtendedTagHash::Hash32(h) => h.fmt(f),
            ExtendedTagHash::Hash64(h) => h.fmt(f),
        }
    }
}

struct ExtendedScanResult {
    pub successful: bool,
    pub file_hashes: Vec<ScannedHashWithEntry<ExtendedTagHash>>,

    /// References from other files
    pub references: Vec<(TagHash, UEntryHeader)>,
}

impl ExtendedScanResult {
    pub fn from_scanresult(s: ScanResult) -> ExtendedScanResult {
        let mut file_hashes_combined = vec![];

        file_hashes_combined.extend(s.file_hashes.into_iter().map(|s| ScannedHashWithEntry {
            offset: s.offset,
            hash: ExtendedTagHash::Hash32(s.hash),
            entry: package_manager().get_entry(s.hash),
        }));

        file_hashes_combined.extend(s.file_hashes64.into_iter().map(|s| {
            ScannedHashWithEntry {
                offset: s.offset,
                hash: ExtendedTagHash::Hash64(s.hash),
                entry: package_manager().get_entry(
                    package_manager()
                        .hash64_table
                        .get(&s.hash.0)
                        .unwrap()
                        .hash32,
                ),
            }
        }));

        file_hashes_combined.sort_unstable_by_key(|v| v.offset);

        ExtendedScanResult {
            successful: s.successful,
            file_hashes: file_hashes_combined,
            references: s
                .references
                .into_iter()
                // TODO(cohae): Unwrap *should* be safe as long as the cache is valid but i want to be sure
                .map(|t| (t, package_manager().get_entry(t).unwrap()))
                .collect(),
        }
    }
}

struct ScannedHashWithEntry<T: Sized> {
    pub offset: u64,
    pub hash: T,
    pub entry: Option<UEntryHeader>,
}

/// Traverses down every tag to make a hierarchy of tags
fn traverse_tags(
    starting_tag: TagHash,
    depth_limit: usize,
    cache: Arc<TagCache>,
    show_strings: bool,
) -> String {
    let mut result = String::new();
    let mut seen_tags = Default::default();
    let mut pipe_stack = vec![];

    traverse_tag(
        &mut result,
        starting_tag,
        TagHash::NONE,
        0,
        &mut seen_tags,
        &mut pipe_stack,
        depth_limit,
        cache,
        show_strings,
    );

    result
}

#[allow(clippy::too_many_arguments)]
fn traverse_tag(
    out: &mut String,
    tag: TagHash,
    parent_tag: TagHash,
    offset: u64,
    seen_tags: &mut IntSet<TagHash>,
    pipe_stack: &mut Vec<char>,
    depth_limit: usize,
    cache: Arc<TagCache>,
    show_strings: bool,
) {
    let depth = pipe_stack.len();

    let pm = package_manager();

    seen_tags.insert(tag);

    let entry = pm.get_entry(tag);
    let fancy_tag = format_tag_entry(tag, entry.as_ref());
    writeln!(out, "{fancy_tag} @ 0x{offset:X}",).ok();

    if depth >= depth_limit {
        let mut line_header = String::new();
        for s in pipe_stack.iter() {
            write!(line_header, "{s}   ").ok();
        }

        writeln!(out, "{line_header}└ Depth limit reached ({})", depth_limit).ok();

        return;
    }

    let Some(scan_result) = cache.hashes.get(&tag).cloned() else {
        return;
    };

    let scan = ExtendedScanResult::from_scanresult(scan_result);

    // writeln!(
    //     out,
    //     "{} {} ({}+{}, ref {}) @ 0x{offset:X}",
    //     tag,
    //     TagType::from_type_subtype(entry.file_type, entry.file_subtype),
    //     entry.file_type,
    //     entry.file_subtype,
    //     TagHash(entry.reference),
    // )
    // .ok();

    let all_hashes = scan
        .file_hashes
        .iter()
        .map(|v| (v.hash.hash32(), v.offset))
        .collect_vec();

    if all_hashes.is_empty() {
        return;
    }

    let mut line_header = String::new();
    for s in pipe_stack.iter() {
        write!(line_header, "{s}   ").ok();
    }

    if show_strings {
        let tag_data = package_manager().read_tag(tag).unwrap();
        let mut raw_strings = vec![];
        for (i, b) in tag_data.chunks_exact(4).enumerate() {
            let v: [u8; 4] = b.try_into().unwrap();
            let hash = u32::from_le_bytes(v);

            if hash == 0x80800065 {
                raw_strings.extend(read_raw_string_blob(&tag_data, i as u64 * 4));
            }
        }

        if !raw_strings.is_empty() {
            writeln!(
                out,
                "{line_header}├──Strings: [{}]",
                raw_strings.into_iter().map(|(_, string)| string).join(", ")
            )
            .ok();
        }
    }

    for (i, (t, offset)) in all_hashes.iter().enumerate() {
        let branch = if i + 1 == all_hashes.len() {
            "└"
        } else {
            "├"
        };

        // Last tag, add a space instead of a pipe
        if i + 1 == all_hashes.len() {
            pipe_stack.push(' ');
        } else {
            pipe_stack.push('│');
        }

        if seen_tags.contains(t) {
            let entry = pm.get_entry(*t);
            let fancy_tag = format_tag_entry(*t, entry.as_ref());

            let offset_label = if *offset == u64::MAX {
                "TagHeader reference".to_string()
            } else {
                format!("0x{:X}", offset)
            };

            if entry
                .map(|e| e.file_type != 8 && e.file_subtype != 16)
                .unwrap_or_default()
            {
                writeln!(out, "{line_header}{branch}──{fancy_tag} @ {offset_label}").ok();
            } else if *t == parent_tag {
                writeln!(
                    out,
                    "{line_header}{branch}──{fancy_tag} @ {offset_label} (parent)"
                )
                .ok();
            } else if *t == tag {
                writeln!(
                    out,
                    "{line_header}{branch}──{fancy_tag} @ {offset_label} (self reference)"
                )
                .ok();
            } else {
                writeln!(
                    out,
                    "{line_header}{branch}──{fancy_tag} @ {offset_label} (already traversed)"
                )
                .ok();
            }
        } else {
            write!(out, "{line_header}{branch}──").ok();
            traverse_tag(
                out,
                *t,
                tag,
                *offset,
                seen_tags,
                pipe_stack,
                depth_limit,
                cache.clone(),
                show_strings,
            );
        }
        pipe_stack.pop();
    }

    writeln!(out, "{line_header}").ok();
}

fn read_raw_string_blob(data: &[u8], offset: u64) -> Vec<(u64, String)> {
    let mut strings = vec![];

    let mut c = Cursor::new(data);
    (|| {
        c.seek(SeekFrom::Start(offset + 4))?;
        let (buffer_size, buffer_base_offset) = if package_manager().version.is_d1() {
            let buffer_size: u32 = c.read_be()?;
            let buffer_base_offset = offset + 4 + 4;
            (buffer_size as u64, buffer_base_offset)
        } else {
            let buffer_size: u64 = c.read_le()?;
            let buffer_base_offset = offset + 4 + 8;
            (buffer_size, buffer_base_offset)
        };

        let mut buffer = vec![0u8; buffer_size as usize];
        c.read_exact(&mut buffer)?;

        let mut s = String::new();
        let mut string_start = 0_u64;
        for (i, b) in buffer.into_iter().enumerate() {
            match b as char {
                '\0' => {
                    if !s.is_empty() {
                        strings.push((buffer_base_offset + string_start, s.clone()));
                        s.clear();
                    }

                    string_start = i as u64 + 1;
                }
                c => s.push(c),
            }
        }

        if !s.is_empty() {
            strings.push((buffer_base_offset + string_start, s));
        }

        <anyhow::Result<()>>::Ok(())
    })()
    .ok();

    strings
}

impl Drop for TagView {
    fn drop(&mut self) {
        for (_, (_, t)) in self.textures.iter() {
            self.render_state.renderer.write().free_texture(t)
        }
    }
}

pub fn format_tag_entry(tag: TagHash, entry: Option<&UEntryHeader>) -> String {
    if let Some(entry) = entry {
        let named_tag = package_manager()
            .named_tags
            .iter()
            .find(|v| v.hash == tag)
            .map(|v| format!("{} ", v.name))
            .unwrap_or_default();

        let ref_label = REFERENCE_MAP
            .read()
            .get(&entry.reference)
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();

        format!(
            "{named_tag}{tag} {}{ref_label} ({}+{}, ref {})",
            TagType::from_type_subtype(entry.file_type, entry.file_subtype),
            entry.file_type,
            entry.file_subtype,
            TagHash(entry.reference),
        )
    } else {
        format!("{} (pkg entry not found)", tag)
    }
}

#[binread]
struct TagArray {
    pub count: u64,
    pub tagtype: u32,

    #[br(ignore)]
    pub references: Vec<u64>,
}
