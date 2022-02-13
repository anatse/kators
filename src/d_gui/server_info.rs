use crate::kafka::{KafkaServer, Password};
use eframe::egui;
use eframe::egui::CursorIcon::Default;
use eframe::egui::{Label, TextEdit, Ui};

#[derive(Debug)]
pub struct ServerInfo {
    server: KafkaServer,
    ssl_keystore_location: String,
    password: String,
}

impl ServerInfo {
    pub fn from_server(server: &KafkaServer) -> Self {
        Self {
            server: server.clone(),
            ssl_keystore_location: server
                .ssl_keystore_location
                .clone()
                .unwrap_or("".to_string()),
            password: "".to_string(),
        }
    }

    pub fn set_server(&mut self, server: &KafkaServer) -> &mut Self {
        if &self.server != server {
            self.server = server.clone();
        }
        self
    }

    pub fn new() -> Self {
        Self {
            server: KafkaServer {
                name: "".to_string(),
                bootstrap: "".to_string(),
                ssl_verification: false,
                ssl_keystore_location: None,
                ssl_keystore_password: None,
                ssl_security_protocol: "PLAINTEXT".to_string(),
                message_max_bytes: 100,
                fetch_max_bytes: 102400,
            },
            ssl_keystore_location: "".to_string(),
            password: "".to_string(),
        }
    }

    pub fn server(&mut self) -> KafkaServer {
        let mut ks = self.server.clone();
        if self.ssl_keystore_location.len() > 0 {
            ks.ssl_keystore_location = Some(self.ssl_keystore_location.clone());
        }

        if self.password.len() > 0 {
            ks.ssl_keystore_password = Some(Password::new(&self.password).unwrap());
        }

        ks
    }

    pub fn ui(&mut self, ui: &mut Ui, enabled: bool) {
        ui.label("Server name");
        ui.add(
            egui::TextEdit::singleline(&mut self.server.name)
                .interactive(enabled)
                .hint_text("server name"),
        )
        .on_hover_text("Define server name");
        ui.end_row();

        ui.label("Bootstrap");
        ui.add(
            egui::TextEdit::singleline(&mut self.server.bootstrap)
                .interactive(enabled)
                .hint_text("bootstrap"),
        )
        .on_hover_text("Define bootstrap servers");
        ui.end_row();

        ui.label("SSL Certificate verification")
            .on_hover_text("SSL Certificate verification");
        ui.add_enabled(
            enabled,
            egui::Checkbox::new(&mut self.server.ssl_verification, "Verify"),
        );
        ui.end_row();

        ui.label("Fetch max bytes");
        ui.add_enabled(
            enabled,
            egui::DragValue::new(&mut self.server.fetch_max_bytes).speed(1.0),
        );
        ui.end_row();

        ui.label("Message max bytes");
        ui.add_enabled(
            enabled,
            egui::DragValue::new(&mut self.server.message_max_bytes).speed(1.0),
        );
        ui.end_row();

        ui.label("Security protocol");
        egui::ComboBox::from_label("")
            .selected_text(self.server.ssl_security_protocol.clone())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut self.server.ssl_security_protocol,
                    "PLAINTEXT".to_string(),
                    "PLAINTEXT".to_string(),
                );
                ui.selectable_value(
                    &mut self.server.ssl_security_protocol,
                    "SSL".to_string(),
                    "SSL".to_string(),
                );
            });
        ui.end_row();

        ui.label("Keystore location");
        ui.add(
            egui::TextEdit::singleline(&mut self.ssl_keystore_location)
                .interactive(enabled)
                .hint_text("keystore"),
        )
        .on_hover_text("Define keystore location");
        ui.end_row();

        ui.label("Keystore password");
        ui.add(
            egui::TextEdit::singleline(&mut self.password)
                .interactive(enabled)
                .hint_text("password"),
        )
        .on_hover_text("Define keystore password");
        ui.end_row();
    }
}
