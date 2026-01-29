use std::{cell::RefCell, sync::Arc};

use ahash::HashMap;
use alkahest_data::activity::SActivity;
use egui::{AtomExt, Atoms, Color32, ImageSource, IntoAtoms, Vec2, vec2};
use google_material_symbols::GoogleMaterialSymbols;
use itertools::Itertools;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    app::SharedState,
    ui::{
        icons,
        tabs::{Tab, TabResult, activity::ActivityTab},
        util::DButton,
    },
};

pub struct ActivityListTab {
    shared_state: Arc<SharedState>,
    root_node: ActivityTreeNode,

    all_nodes: Vec<ActivityTreeNode>,

    current_node: RefCell<Vec<usize>>,

    search_query: RefCell<String>,
}

impl ActivityListTab {
    pub fn new(shared_state: &Arc<SharedState>) -> Self {
        let mut crucible_nodes = HashMap::<String, Vec<ActivityTreeNode>>::default();
        let mut gambit_nodes = vec![];
        let mut destination_nodes = HashMap::<String, Vec<ActivityTreeNode>>::default();
        let mut dungeon_nodes = vec![];
        let mut raid_nodes = vec![];
        let mut strike_nodes = vec![];
        let mut patrol_nodes = vec![];

        let mut all_nodes = HashMap::default();

        for (activity_string, tag) in
            package_manager().get_named_tags_by_class(SActivity::ID.unwrap())
        {
            let Some((destination, activity)) = activity_string.split_once(".") else {
                continue;
            };

            all_nodes.insert(
                activity_string.clone(),
                ActivityTreeNode::Leaf {
                    title: format!("{activity} ({destination})"),
                    tag,
                },
            );

            if activity.ends_with("_ambient") {
                continue;
            }

            if destination.starts_with("crucible_") {
                crucible_nodes
                    .entry(destination.to_string())
                    .or_default()
                    .push(ActivityTreeNode::Leaf {
                        title: activity.to_string(),
                        tag,
                    });
            } else if destination.starts_with("gambit_") {
                gambit_nodes.push(ActivityTreeNode::Leaf {
                    title: destination.to_string(),
                    tag,
                });
            } else {
                let leaf = ActivityTreeNode::Leaf {
                    title: activity.to_string(),
                    tag,
                };
                let kind = leaf.kind();
                destination_nodes
                    .entry(destination.to_string())
                    .or_default()
                    .push(leaf.clone());

                match kind {
                    Some(ActivityKind::Dungeon) => {
                        dungeon_nodes.push(leaf);
                    }
                    Some(ActivityKind::Raid) => {
                        raid_nodes.push(leaf.clone());
                    }
                    Some(ActivityKind::Strike) => {
                        strike_nodes.push(leaf.clone());
                    }
                    Some(ActivityKind::Patrol) => {
                        patrol_nodes.push(leaf.clone());
                    }
                    _ => {}
                }
            }
        }

        let mut crucible_nodes = crucible_nodes
            .into_iter()
            .map(|(destination, mut activities)| {
                activities.sort_by_key(|activity| activity.title().to_string());
                ActivityTreeNode::Branch {
                    title: destination,
                    children: activities,
                }
            })
            .collect_vec();

        let mut destination_nodes = destination_nodes
            .into_iter()
            .map(|(destination, mut activities)| {
                activities.sort_by_key(|activity| activity.title().to_string());
                ActivityTreeNode::Branch {
                    title: destination,
                    children: activities,
                }
            })
            .collect_vec();

        crucible_nodes.sort_by_key(|node| node.title().to_string());
        destination_nodes.sort_by_key(|node| node.title().to_string());
        gambit_nodes.sort_by_key(|node| node.title().to_string());
        raid_nodes.sort_by_key(|node| node.title().to_string());
        strike_nodes.sort_by_key(|node| node.title().to_string());
        destination_nodes.sort_by_key(|node| node.title().to_string());
        patrol_nodes.sort_by_key(|node| node.title().to_string());

        let mut all_nodes = all_nodes.into_values().collect_vec();
        all_nodes.sort_by_key(|node| node.title().to_string());

        Self {
            shared_state: shared_state.clone(),
            root_node: ActivityTreeNode::Branch {
                title: String::new(),
                children: vec![
                    ActivityTreeNode::Branch {
                        title: format!("{} Packages", GoogleMaterialSymbols::Package2),
                        children: destination_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Crucible".to_string(),
                        children: crucible_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Gambit".to_string(),
                        children: gambit_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Raids".to_string(),
                        children: raid_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Dungeons".to_string(),
                        children: dungeon_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Strikes".to_string(),
                        children: strike_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Patrol".to_string(),
                        children: patrol_nodes,
                    },
                ],
            },
            all_nodes,
            search_query: String::new().into(),
            current_node: vec![].into(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> TabResult {
        let mut result = TabResult::Continue;
        self.node_children_ui(ui, &self.root_node, 0, &mut result);
        result
    }

    /// Draws a branch node's children
    fn node_children_ui(
        &self,
        ui: &mut egui::Ui,
        node: &ActivityTreeNode,
        depth: usize,
        result: &mut TabResult,
    ) {
        let ActivityTreeNode::Branch { children, .. } = node else {
            ui.label("TODO: leaf in root nodes");
            return;
        };

        let current_selected = self.current_node.borrow().get(depth).copied();
        egui::SidePanel::left(format!("activity_node_depth{depth}")).show_inside(ui, |ui| {
            if depth == 0 {
                ui.add(
                    egui::TextEdit::singleline(&mut *self.search_query.borrow_mut())
                        .hint_text("Search"),
                );
            }

            if depth == 0 && !self.search_query.borrow().is_empty() {
                egui::ScrollArea::vertical()
                    .id_salt("activity_list_nodes_search")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        let query = &*self.search_query.borrow();
                        for child in self
                            .all_nodes
                            .iter()
                            .filter(|child| child.title().to_lowercase().contains(query))
                        {
                            #[allow(clippy::collapsible_if)]
                            if let ActivityTreeNode::Leaf { title, tag } = child {
                                if DButton::new(child.atoms())
                                    .min_size(vec2(768.0, 32.0))
                                    .stroke(1.0, child.stroke_color())
                                    .fill(child.bg_color())
                                    .ui(ui)
                                    .clicked()
                                {
                                    match ActivityTab::new(
                                        &self.shared_state,
                                        *tag,
                                        title.to_string(),
                                    ) {
                                        Ok(tab) => {
                                            *result = TabResult::Open(Tab::Activity(tab));
                                        }
                                        Err(err) => {
                                            // TODO(cohae): Error popup
                                            error!("Failed to open activity tab: {}", err);
                                        }
                                    }
                                }
                            }
                        }
                    });
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("activity_list_nodes")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        for (i, child) in children.iter().enumerate() {
                            match child {
                                ActivityTreeNode::Branch { .. } => {
                                    let btn = if Some(i) == current_selected {
                                        DButton::new_white(child.atoms())
                                    } else {
                                        DButton::new(child.atoms()).fill(child.bg_color())
                                    }
                                    .stroke(1.0, child.stroke_color())
                                    .min_size(vec2(512.0, 32.0))
                                    .ui(ui);

                                    if btn.clicked() {
                                        self.current_node.borrow_mut().truncate(depth);
                                        self.current_node.borrow_mut().push(i);
                                    }
                                }
                                ActivityTreeNode::Leaf { title, tag } => {
                                    if DButton::new((child.atoms(), format!("({tag})")))
                                        .min_size(vec2(512.0, 32.0))
                                        .stroke(1.0, child.stroke_color())
                                        .fill(child.bg_color())
                                        .ui(ui)
                                        .clicked()
                                    {
                                        match ActivityTab::new(
                                            &self.shared_state,
                                            *tag,
                                            title.to_string(),
                                        ) {
                                            Ok(tab) => {
                                                *result = TabResult::Open(Tab::Activity(tab));
                                            }
                                            Err(err) => {
                                                // TODO(cohae): Error popup
                                                error!("Failed to open activity tab: {}", err);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
            }
        });

        let next_node = self.current_node.borrow().get(depth).copied();
        if let Some(next_node) = next_node {
            self.node_children_ui(ui, &children[next_node], depth + 1, result);
        }
    }
}

#[derive(Clone)]
enum ActivityTreeNode {
    Leaf {
        title: String,
        tag: TagHash,
    },
    Branch {
        title: String,
        children: Vec<ActivityTreeNode>,
    },
}

impl ActivityTreeNode {
    fn title(&self) -> &str {
        match self {
            ActivityTreeNode::Leaf { title, .. } => title,
            ActivityTreeNode::Branch { title, .. } => title,
        }
    }

    fn kind(&self) -> Option<ActivityKind> {
        let title = self.title();

        let kind = match title.to_lowercase().as_str() {
            v if v.starts_with("crucible") => ActivityKind::Crucible,
            v if v.starts_with("raid") || v.contains("raid_") => ActivityKind::Raid,
            v if v.starts_with("iron_banner") => ActivityKind::IronBanner,
            v if v.starts_with("trials") => ActivityKind::Trials,
            v if v.starts_with("gambit") => ActivityKind::Gambit,
            v if v.starts_with("dungeon") => ActivityKind::Dungeon,
            v if v.starts_with("mission_") => ActivityKind::Mission,
            v if v.starts_with("quest") => ActivityKind::Quest,
            v if v.starts_with("strike") => ActivityKind::Strike,
            v if v.starts_with("exotic") => ActivityKind::Exotic,
            v if v.contains("freeroam") => ActivityKind::Patrol,
            "patrol" => ActivityKind::Patrol,
            v if v.contains("_ls_a") || v.contains("_ls_b") || v.contains("_ls_c") => {
                ActivityKind::LostSector
            }
            _ => return None,
        };

        Some(kind)
    }

    fn atoms<'a>(&'a self) -> Atoms<'a> {
        let title = self.title();

        if let Some(kind) = self.kind() {
            (
                kind.icon().atom_size(Vec2::splat(32.0)),
                "",
                title.to_string(),
            )
                .into_atoms()
        } else {
            title.into_atoms()
        }
    }

    fn color(&self) -> Color32 {
        self.kind().map(|k| k.color()).unwrap_or(Color32::WHITE)
    }

    fn stroke_color(&self) -> Color32 {
        self.color().gamma_multiply(1.7)
    }

    fn bg_color(&self) -> Color32 {
        let c = self.color();
        if c == Color32::WHITE {
            Color32::TRANSPARENT
        } else {
            c.gamma_multiply(0.33)
        }
    }
}

enum ActivityKind {
    Crucible,
    Raid,
    IronBanner,
    Trials,
    Gambit,
    Dungeon,
    Mission,
    Quest,
    Strike,
    Exotic,
    Patrol,
    LostSector,
}

impl ActivityKind {
    fn icon(&self) -> ImageSource<'static> {
        match self {
            ActivityKind::Crucible => icons::CRUCIBLE,
            ActivityKind::Raid => icons::RAID,
            ActivityKind::IronBanner => icons::IRON_BANNER,
            ActivityKind::Trials => icons::OSIRIS,
            ActivityKind::Gambit => icons::GAMBIT,
            ActivityKind::Dungeon => icons::DUNGEON,
            ActivityKind::Patrol => icons::PATROL,
            ActivityKind::LostSector => icons::LOST_SECTOR,
            ActivityKind::Strike => icons::STRIKE,
            ActivityKind::Exotic => icons::ENGRAM,
            ActivityKind::Quest => icons::QUEST,
            ActivityKind::Mission => icons::QUEST,
        }
    }

    fn color(&self) -> Color32 {
        match self {
            ActivityKind::Crucible => Color32::from_rgb(145, 37, 29),
            ActivityKind::Raid => Color32::WHITE,
            ActivityKind::IronBanner => Color32::WHITE,
            ActivityKind::Trials => Color32::from_rgb(198, 159, 99),
            ActivityKind::Gambit => Color32::from_rgb(57, 119, 94),
            ActivityKind::Dungeon => Color32::from_rgb(104, 85, 72),
            ActivityKind::Patrol => Color32::WHITE,
            ActivityKind::LostSector => Color32::from_rgb(80, 73, 159),
            ActivityKind::Strike => Color32::from_rgb(57, 100, 128),
            ActivityKind::Exotic => Color32::from_rgb(191, 153, 65),
            ActivityKind::Quest | ActivityKind::Mission => Color32::from_rgb(38, 68, 127),
        }
    }
}
