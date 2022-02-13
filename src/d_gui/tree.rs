use crate::d_gui::model::{KafkaTreeBroker, KafkaTreePartition, KafkaTreeTopic};
use crate::kafka::KafkaServer;
use anyhow::Result as AnyResult;
use eframe::egui::{CollapsingHeader, CollapsingResponse, RichText, SelectableLabel, Ui};
use log::{error, info};
use std::fmt::{Debug, Display, Formatter};

/// Tree widget with kafka server information
#[derive(Clone, Debug)]
pub struct Tree {
    selected_node: Option<TreeNode>,
    child: SubTree,
}

impl Tree {
    pub fn from_servers(servers: &Vec<KafkaServer>) -> Self {
        let subtrees = servers
            .iter()
            .map(|srv| SubTree::new(TreeNode::Server(srv.clone(), false)))
            .collect::<Vec<SubTree>>();

        Self {
            selected_node: None,
            child: SubTree {
                children: subtrees,
                node: TreeNode::Folder("Clusters".to_string()),
            },
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.child.ui(ui, &mut self.selected_node);
    }

    pub fn selected(&self) -> Option<&TreeNode> {
        self.selected_node.as_ref()
    }

    pub fn refresh_server(&mut self, name: String) {
        self.child.children = self
            .child
            .children
            .iter()
            .map(|c| match &c.node {
                TreeNode::Server(server, conn) => {
                    if server.name == name {
                        self.selected_node = None;
                        SubTree::new(TreeNode::Server(server.clone(), false))
                    } else {
                        c.clone()
                    }
                }
                _ => c.clone(),
            })
            .collect();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeNode {
    Folder(String),
    Server(KafkaServer, bool),
    Broker(KafkaTreeBroker),
    Topic(KafkaTreeTopic),
    Partition(KafkaTreePartition),
}

impl Display for TreeNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeNode::Folder(internal) => f.write_str(&internal.to_string()),
            TreeNode::Server(internal, _) => f.write_str(&internal.to_string()),
            TreeNode::Broker(internal) => f.write_str(&internal.to_string()),
            TreeNode::Topic(internal) => f.write_str(&internal.to_string()),
            TreeNode::Partition(internal) => f.write_str(&internal.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SubTree {
    node: TreeNode,
    children: Vec<SubTree>,
}

impl SubTree {
    #[inline]
    pub fn new(node: TreeNode) -> Self {
        Self {
            children: vec![],
            node,
        }
    }

    #[inline]
    pub fn with_children(node: TreeNode, children: Vec<SubTree>) -> Self {
        Self { children, node }
    }

    #[inline]
    fn draw_collapsing_header(
        &mut self,
        ui: &mut Ui,
        selected_node: &mut Option<TreeNode>,
    ) -> CollapsingResponse<()> {
        CollapsingHeader::new(format!("{}", &self.node))
            .default_open(false)
            .selectable(true)
            .selected(
                selected_node
                    .as_ref()
                    .map(|sn| sn == &self.node)
                    .unwrap_or(false),
            )
            .show(ui, |ui| self.children_ui(ui, selected_node))
    }

    #[inline]
    fn folder_draw(
        &mut self,
        ui: &mut Ui,
        selected_node: &mut Option<TreeNode>,
    ) -> CollapsingResponse<()> {
        self.draw_collapsing_header(ui, selected_node)
    }

    /// Draws full servers tree
    #[inline]
    fn server_draw(
        &mut self,
        ui: &mut Ui,
        selected_node: &mut Option<TreeNode>,
    ) -> AnyResult<CollapsingResponse<()>> {
        let srv = self.draw_collapsing_header(ui, selected_node);
        if srv.header_response.clicked() {
            *selected_node = Some(self.node.clone());
            if let TreeNode::Server(server, connected) = &self.node {
                if !connected {
                    info!("trying to connect to server");
                    let consumer = server.open()?.create_consumer()?;
                    let md = consumer.read_metadata()?;
                    let mut brokers = md
                        .brokers()
                        .iter()
                        .map(|broker| {
                            SubTree::new(TreeNode::Broker(KafkaTreeBroker::from_md(
                                server.name.clone(),
                                broker,
                            )))
                        })
                        .collect::<Vec<SubTree>>();
                    brokers.sort_by(|a, b| a.node.to_string().cmp(&b.node.to_string()));

                    let mut topics = md
                        .topics()
                        .iter()
                        .map(|topic| {
                            let t_topic = KafkaTreeTopic::from_md(server.name.clone(), topic);
                            let partitions = t_topic
                                .partitions()
                                .iter()
                                .map(|p| SubTree::new(TreeNode::Partition(p.clone())))
                                .collect::<Vec<SubTree>>();

                            SubTree::with_children(
                                TreeNode::Topic(t_topic),
                                vec![SubTree::with_children(
                                    TreeNode::Folder("Partition".to_string()),
                                    partitions,
                                )],
                            )
                        })
                        .collect::<Vec<SubTree>>();
                    topics.sort_by(|a, b| a.node.to_string().cmp(&b.node.to_string()));
                    if !self.children.is_empty() {
                        self.children.clear();
                    }

                    self.children = vec![
                        SubTree::with_children(TreeNode::Folder("Brokers".to_string()), brokers),
                        SubTree::with_children(TreeNode::Folder("Topics".to_string()), topics),
                    ];

                    self.node = TreeNode::Server(server.clone(), true);
                }
            }
        }

        Ok(srv)
    }

    #[inline]
    fn topic_draw(
        &mut self,
        ui: &mut Ui,
        selected_node: &mut Option<TreeNode>,
    ) -> CollapsingResponse<()> {
        let response = self.draw_collapsing_header(ui, selected_node);
        if response.header_response.clicked() {
            *selected_node = Some(self.node.clone());
        }
        response
    }

    #[inline]
    fn draw_simple(&mut self, ui: &mut Ui, selected_node: &mut Option<TreeNode>, name: String) {
        if ui
            .add(SelectableLabel::new(
                selected_node
                    .as_ref()
                    .map(|sn| sn == &self.node)
                    .unwrap_or(false),
                RichText::new(name),
            ))
            .clicked()
        {
            *selected_node = Some(self.node.clone());
        }
    }

    pub fn ui(&mut self, ui: &mut Ui, selected_node: &mut Option<TreeNode>) {
        match &self.node {
            TreeNode::Folder(_) => {
                let _ = self.folder_draw(ui, selected_node);
            }
            TreeNode::Server(_, _) => match self.server_draw(ui, selected_node) {
                Ok(_) => {}
                Err(err) => error!("Error connecting to server {}", err),
            },
            TreeNode::Broker(broker) => {
                let bs = broker.to_string();
                self.draw_simple(ui, selected_node, bs);
            }
            TreeNode::Topic(_) => {
                let _ = self.topic_draw(ui, selected_node);
            }
            TreeNode::Partition(partition) => {
                let ps = partition.to_string();
                self.draw_simple(ui, selected_node, ps);
            }
        };
    }

    #[inline]
    fn children_ui(&mut self, ui: &mut Ui, selected_node: &mut Option<TreeNode>) {
        self.children = self
            .children
            .clone()
            .into_iter()
            .map(|mut tree| {
                tree.ui(ui, selected_node);
                tree
            })
            .collect();
    }
}
