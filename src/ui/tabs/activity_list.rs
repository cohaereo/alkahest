use std::{cell::RefCell, sync::Arc};

use ahash::HashMap;
use alkahest_data::activity::SActivity;
use egui::vec2;
use itertools::Itertools;
use tiger_parse::TigerReadable;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    app::SharedState,
    ui::{
        tabs::{Tab, TabResult, activity::ActivityTab},
        util::DButton,
    },
};

pub struct ActivityListTab {
    shared_state: Arc<SharedState>,
    root_node: ActivityTreeNode,

    current_node: RefCell<Vec<usize>>,
}

impl ActivityListTab {
    pub fn new(shared_state: &Arc<SharedState>) -> Self {
        let mut crucible_nodes = HashMap::<String, Vec<ActivityTreeNode>>::default();
        let mut gambit_nodes = vec![];
        let mut destination_nodes = HashMap::<String, Vec<ActivityTreeNode>>::default();

        for (activity_string, tag) in
            package_manager().get_named_tags_by_class(SActivity::ID.unwrap())
        {
            let Some((destination, activity)) = activity_string.split_once(".") else {
                continue;
            };

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
                destination_nodes
                    .entry(destination.to_string())
                    .or_default()
                    .push(ActivityTreeNode::Leaf {
                        title: activity.to_string(),
                        tag,
                    });
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

        Self {
            shared_state: shared_state.clone(),
            root_node: ActivityTreeNode::Branch {
                title: String::new(),
                children: vec![
                    ActivityTreeNode::Branch {
                        title: "Crucible".to_string(),
                        children: crucible_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Gambit".to_string(),
                        children: gambit_nodes,
                    },
                    ActivityTreeNode::Branch {
                        title: "Destinations".to_string(),
                        children: destination_nodes,
                    },
                ],
            },
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
            egui::ScrollArea::vertical()
                .id_salt("activity_list_nodes")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                    for (i, child) in children.iter().enumerate() {
                        match child {
                            ActivityTreeNode::Branch { title, .. } => {
                                let btn = if Some(i) == current_selected {
                                    DButton::new_white(title)
                                } else {
                                    DButton::new(title)
                                }
                                .min_size(vec2(512.0, 32.0))
                                .ui(ui);

                                if btn.clicked() {
                                    self.current_node.borrow_mut().truncate(depth);
                                    self.current_node.borrow_mut().push(i);
                                }
                            }
                            ActivityTreeNode::Leaf { title, tag } => {
                                if DButton::new((title, format!("({tag})")))
                                    .min_size(vec2(512.0, 32.0))
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
        });

        let next_node = self.current_node.borrow().get(depth).copied();
        if let Some(next_node) = next_node {
            self.node_children_ui(ui, &children[next_node], depth + 1, result);
        }
    }
}

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
}
