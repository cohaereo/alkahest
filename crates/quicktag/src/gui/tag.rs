use std::{
    fmt::Display,
    io::{Cursor, Read, Seek, SeekFrom},
    sync::Arc,
    time::{Duration, Instant},
};

use binrw::BinReaderExt;
use destiny_pkg::{package::UEntryHeader, TagHash, TagHash64};
use eframe::{
    egui::{self, RichText},
    epaint::Color32,
};
use itertools::Itertools;
use log::error;
use nohash_hasher::IntSet;
use poll_promise::Promise;
use std::fmt::Write;

use crate::{
    packages::package_manager,
    references::REFERENCE_MAP,
    scanner::{ScanResult, ScannedHash, TagCache},
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

    tag: TagHash,
    // tag_data: Vec<u8>,
    tag_entry: UEntryHeader,

    scan: ExtendedScanResult,
    tag_traversal: Option<Promise<String>>,
    traversal_depth_limit: usize,
    // traversal_show_strings: bool,
    start_time: Instant,
}

impl TagView {
    pub fn create(
        cache: Arc<TagCache>,
        string_cache: Arc<StringCache>,
        tag: TagHash,
    ) -> Option<TagView> {
        let tag_data = package_manager().read_tag(tag).unwrap();
        let mut raw_string_offsets = vec![];
        let mut string_hashes = vec![];
        for (i, b) in tag_data.chunks_exact(4).enumerate() {
            let v: [u8; 4] = b.try_into().unwrap();
            let hash = u32::from_le_bytes(v);

            if hash == 0x80800065 {
                raw_string_offsets.push(i as u64 * 4);
            }

            if string_cache.contains_key(&hash) {
                string_hashes.push((i as u64 * 4, hash));
            }
        }

        let raw_strings = raw_string_offsets
            .into_iter()
            .flat_map(|o| read_raw_string_blob(&tag_data, o))
            .collect_vec();

        Some(Self {
            string_hashes,
            tag,
            // tag_data,
            tag_entry: package_manager().get_entry(tag).unwrap(),

            scan: ExtendedScanResult::from_scanresult(cache.get(&tag).cloned()?),
            cache,
            traversal_depth_limit: 16,
            tag_traversal: None,
            // traversal_show_strings: false,
            string_cache,
            raw_strings,
            start_time: Instant::now(),
        })
    }

    /// Replaces this view with another tag
    pub fn open_tag(&mut self, tag: TagHash) {
        if let Some(tv) = Self::create(self.cache.clone(), self.string_cache.clone(), tag) {
            *self = tv;
        } else {
            error!("Could not open new tag view for {tag} (tag not found in cache)");
        }
    }
}

impl View for TagView {
    fn view(&mut self, ctx: &eframe::egui::Context, ui: &mut eframe::egui::Ui) {
        let mut open_new_tag = None;

        let ref_label = REFERENCE_MAP
            .read()
            .get(&self.tag_entry.reference)
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();
        ui.heading(format!(
            "Tag {} {}{ref_label} ({}+{}, ref {})",
            self.tag,
            TagType::from_type_subtype(self.tag_entry.file_type, self.tag_entry.file_subtype),
            self.tag_entry.file_type,
            self.tag_entry.file_subtype,
            TagHash(self.tag_entry.reference)
        ))
        .context_menu(|ui| tag_context(ui, self.tag));

        if ui.button("Open tag data in external application").clicked() {
            open_tag_in_default_application(self.tag);
        }

        ui.separator();
        egui::SidePanel::left("tv_left_panel")
            .resizable(true)
            .min_width(256.0)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(egui::RichText::new("Files referencing this tag").strong());
                    ui.group(|ui| {
                        for (tag, entry) in &self.scan.references {
                            let response = ui.add_enabled(
                                *tag != self.tag,
                                egui::SelectableLabel::new(
                                    false,
                                    format!(
                                        "{tag} {} ({}+{}, ref {})",
                                        TagType::from_type_subtype(
                                            entry.file_type,
                                            entry.file_subtype
                                        ),
                                        entry.file_type,
                                        entry.file_subtype,
                                        TagHash(entry.reference)
                                    ),
                                ),
                            );

                            if response.context_menu(|ui| tag_context(ui, *tag)).clicked() {
                                open_new_tag = Some(*tag);
                            }
                        }
                    });

                    ui.label(egui::RichText::new("Tag references in this file").strong());
                    ui.group(|ui| {
                        for tag in &self.scan.file_hashes {
                            let tag_label = if let Some(entry) = &tag.entry {
                                let tagtype =
                                    TagType::from_type_subtype(entry.file_type, entry.file_subtype);

                                let fancy_tag = format_tag_entry(tag.hash.hash32(), Some(entry));

                                egui::RichText::new(format!("{fancy_tag} @ 0x{:X}", tag.offset))
                                    .color(tagtype.display_color())
                            } else {
                                egui::RichText::new(format!(
                                    "{} (pkg entry not found) @ 0x{:X}",
                                    tag.hash, tag.offset
                                ))
                                .color(Color32::LIGHT_RED)
                            };

                            // TODO(cohae): Highlight/jump to tag in hex viewer
                            let response = ui.add_enabled(
                                tag.hash.hash32() != self.tag,
                                egui::SelectableLabel::new(false, tag_label),
                            );

                            if response
                                .context_menu(|ui| tag_context(ui, tag.hash.hash32()))
                                .clicked()
                            {
                                open_new_tag = Some(tag.hash.hash32());
                            }
                        }
                    });
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
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
                    self.tag_traversal = Some(Promise::spawn_thread("traverse tags", move || {
                        traverse_tags(tag, depth_limit, cache)
                    }));
                }

                if ui.button("Copy traversal").clicked() {
                    if let Some(traversal) = self.tag_traversal.as_ref() {
                        if let Some(result) = traversal.ready() {
                            ui.output_mut(|o| o.copied_text = result.clone());
                        }
                    }
                }

                ui.add(egui::DragValue::new(&mut self.traversal_depth_limit).clamp_range(4..=256));
                ui.label("Max depth");

                // ui.checkbox(&mut self.traversal_show_strings, "Find strings (slow)");
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
        });

        egui::SidePanel::right("tv_right_panel")
            .resizable(true)
            .min_width(320.0)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap = Some(false);
                ui.label(egui::RichText::new("String Hashes").strong());
                ui.group(|ui| {
                    for (offset, hash) in &self.string_hashes {
                        if let Some(strings) = self.string_cache.get(hash) {
                            if strings.len() > 1 {
                                ui.selectable_label(
                                    false,
                                    format!(
                                        "'{}' ({} collisions) {:08x} @ 0x{:X}",
                                        strings[(self.start_time.elapsed().as_secs() as usize)
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
                                    format!("'{}' {:08x} @ 0x{:X}", strings[0], hash, offset),
                                )
                                .clicked();
                            }
                        }
                    }
                });

                ui.label(egui::RichText::new("Raw strings (65008080)").strong());
                ui.group(|ui| {
                    for (offset, string) in &self.raw_strings {
                        ui.selectable_label(false, format!("'{}' @ 0x{:X}", string, offset))
                            .context_menu(|ui| {
                                if ui.selectable_label(false, "Copy text").clicked() {
                                    ui.output_mut(|o| o.copied_text = string.clone());
                                    ui.close_menu();
                                }
                            })
                            .clicked();
                    }
                });
            });

        ctx.request_repaint_after(Duration::from_secs(1));

        if let Some(new_tag) = open_new_tag {
            self.open_tag(new_tag)
        }
    }
}

enum ExtendedTagHash {
    Hash32(TagHash),
    Hash64(TagHash64),
}

impl ExtendedTagHash {
    pub fn hash32(&self) -> TagHash {
        match self {
            ExtendedTagHash::Hash32(h) => *h,
            ExtendedTagHash::Hash64(h) => package_manager().hash64_table.get(&h.0).unwrap().hash32,
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
    pub file_hashes: Vec<ScannedHashWithEntry<ExtendedTagHash>>,
    pub string_hashes: Vec<ScannedHash<u32>>,

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
            file_hashes: file_hashes_combined,
            string_hashes: s.string_hashes,
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
fn traverse_tags(starting_tag: TagHash, depth_limit: usize, cache: Arc<TagCache>) -> String {
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

    let Some(scan_result) = cache.get(&tag).cloned() else {
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

            if entry
                .map(|e| e.file_type != 8 && e.file_subtype != 16)
                .unwrap_or_default()
            {
                writeln!(out, "{line_header}{branch}──{fancy_tag} @ 0x{offset:X}").ok();
            } else if *t == parent_tag {
                writeln!(
                    out,
                    "{line_header}{branch}──{fancy_tag} @ 0x{offset:X} (parent)"
                )
                .ok();
            } else if *t == tag {
                writeln!(
                    out,
                    "{line_header}{branch}──{fancy_tag} @ 0x{offset:X} (self reference)"
                )
                .ok();
            } else {
                writeln!(
                    out,
                    "{line_header}{branch}──{fancy_tag} @ 0x{offset:X} (already traversed)"
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
        let buffer_size: u64 = c.read_le()?;
        let buffer_base_offset = offset + 4 + 8;
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

fn format_tag_entry(tag: TagHash, entry: Option<&UEntryHeader>) -> String {
    if let Some(entry) = entry {
        let ref_label = REFERENCE_MAP
            .read()
            .get(&entry.reference)
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();

        format!(
            "{} {}{ref_label} ({}+{}, ref {})",
            tag,
            TagType::from_type_subtype(entry.file_type, entry.file_subtype),
            entry.file_type,
            entry.file_subtype,
            TagHash(entry.reference),
        )
    } else {
        format!("{} (pkg entry not found)", tag)
    }
}
