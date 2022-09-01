mod model;
mod server_info;
mod topic_info;
mod tree;

use crate::d_gui::model::{KafkaTreePartition, KafkaTreeTopic};
use crate::d_gui::server_info::ServerInfo;
use crate::d_gui::topic_info::TopicInfo;
use crate::d_gui::tree::TreeNode;
use crate::kafka::{AdminCommand, KafkaServer};
use eframe::egui::{CentralPanel, Context};
use eframe::{App, Frame};
use eframe::{egui};
use log::info;
use sled::Tree as DbTree;
use std::rc::Rc;
use tree::Tree;

pub struct KatorApp {
    db: Rc<DbTree>,
    servers: Vec<KafkaServer>,
    current_server: Option<KafkaServer>,
    tree: Tree,
    enabled: bool,
    // Server to add
    edit_server: ServerInfo,
    server_opened: bool,
    server_saved: bool,
    // Topics
    topic_info: TopicInfo,
    // Servers
    server_info: ServerInfo,
    // Add topic
    topic_open: bool,
    topic_name: String,
    topic_partitions: i32,
    topic_repl_factor: i32,
    topic_saved: bool,
}

impl KatorApp {
    pub fn new(db: Rc<sled::Tree>, db_topics: Rc<sled::Tree>) -> Self {
        let servers = KafkaServer::all(&db);
        let tree = Tree::from_servers(&servers);

        Self {
            db: db.clone(),
            servers,
            current_server: None,
            tree,
            enabled: true,
            edit_server: ServerInfo::new(),
            server_opened: false,
            server_saved: false,
            topic_info: TopicInfo::new(db.clone(), db_topics.clone()),
            server_info: ServerInfo::new(),
            topic_open: false,
            topic_name: "".to_string(),
            topic_partitions: 0,
            topic_repl_factor: 0,
            topic_saved: false,
        }
    }

    fn reload_servers(&mut self) {
        self.servers = KafkaServer::all(&self.db);
    }
}

impl App for KatorApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        if self.servers.is_empty() {
            self.reload_servers();
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |main_ui| {
            main_ui.horizontal(|ui| {
                ui.label("Servers");
                if ui.button("add").on_hover_text("Add kafka server").clicked() {
                    self.server_opened = true;
                }

                if self.server_saved {
                    self.reload_servers();
                    self.tree = Tree::from_servers(&self.servers);
                    self.server_saved = false;
                    self.server_opened = false;
                }

                if self.server_opened {
                    // Show window with server definition
                    egui::Window::new("Add server")
                        .default_height(500.0)
                        .open(&mut self.server_opened)
                        .show(ctx, |ui| {
                            egui::Grid::new("server_grid")
                                .num_columns(2)
                                .spacing([40.0, 4.0])
                                .striped(true)
                                .show(ui, |ui| self.edit_server.ui(ui, true));

                            ui.separator();
                            if ui.button("Save").clicked() {
                                self.edit_server.server().store(&self.db);
                                self.server_saved = true;
                            }
                        });
                }

                if ui
                    .add_enabled(
                        self.selected_server().is_some(),
                        egui::Button::new("remove"),
                    )
                    .on_hover_text("Remove selected kafka server")
                    .clicked()
                {
                    if let Some(srv) = self.selected_server() {
                        self.db.remove(srv.name);
                        self.reload_servers();
                        self.tree = Tree::from_servers(&self.servers);
                    }
                }
            })
        });

        // Add topics panel into left size
        egui::SidePanel::left("Topics")
            .resizable(true)
            .default_width(300.0)
            .width_range(200.0..=600.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .hscroll(true)
                    .show(ui, |ui| self.tree.ui(ui));
            });

        if self.topic_saved {
            self.topic_saved = false;
            self.topic_open = false;
        }

        // Show server information
        if let Some(server) = self.selected_server() {
            // Show server information
            CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Reconnect").clicked() {
                        self.tree.refresh_server(server.name.clone());
                    }
                    if ui.button("Add topic").clicked() {
                        self.topic_open = true;
                    }

                    egui::Window::new("Add topic")
                        .open(&mut self.topic_open)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label("Name");
                            ui.add(egui::TextEdit::singleline(&mut self.topic_name));
                            ui.end_row();

                            ui.label("Partitions");
                            ui.add(egui::DragValue::new(&mut self.topic_partitions));
                            ui.end_row();

                            ui.label("Replication factor");
                            ui.add(egui::DragValue::new(&mut self.topic_repl_factor));
                            ui.end_row();
                            ui.separator();
                            ui.end_row();

                            if ui.button("Save").clicked() {
                                self.topic_info.set_topic(&KafkaTreeTopic {
                                    server_name: server.name.clone(),
                                    name: self.topic_name.clone(),
                                    partitions: vec![],
                                });
                                self.topic_info
                                    .send_admin_command(AdminCommand::CreateTopic(
                                        self.topic_name.clone(),
                                        self.topic_partitions,
                                        self.topic_repl_factor,
                                    ));
                                self.topic_saved = true;
                            }
                        });
                });

                ui.separator();
                egui::Grid::new("server_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| self.server_info.set_server(&server).ui(ui, false));
            });
        }

        // Show topic information
        if let Some(topic) = self.selected_topic() {
            CentralPanel::default().show(ctx, |ui| {
                self.topic_info.set_topic(&topic).ui(ui);
            });
        }
    }

    // fn name(&self) -> &str {
    //     "Kator App"
    // }
}

impl KatorApp {
    fn selected_server(&self) -> Option<KafkaServer> {
        self.tree.selected().and_then(|node| match node {
            TreeNode::Server(server, ..) => Some(server.clone()),
            _ => None,
        })
    }

    fn selected_topic(&self) -> Option<KafkaTreeTopic> {
        self.tree.selected().and_then(|node| match node {
            TreeNode::Topic(topic, ..) => Some(topic.clone()),
            _ => None,
        })
    }

    fn selected_partition(&self) -> Option<KafkaTreePartition> {
        self.tree.selected().and_then(|node| match node {
            TreeNode::Partition(p, ..) => Some(p.clone()),
            _ => None,
        })
    }
}
