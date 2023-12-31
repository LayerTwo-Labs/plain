use crate::app::lib;
use crate::app::App;
use eframe::egui;
use lib::{
    bip300301::bitcoin,
    types::{self, Content, Output},
};

pub struct UtxoCreator {
    utxo_type: UtxoType,
    value: String,
    address: String,
    main_address: String,
    main_fee: String,
}

#[derive(Eq, PartialEq)]
enum UtxoType {
    Regular,
    Withdrawal,
}

impl std::fmt::Display for UtxoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Regular => write!(f, "regular"),
            Self::Withdrawal => write!(f, "withdrawal"),
        }
    }
}

impl Default for UtxoCreator {
    fn default() -> Self {
        Self {
            value: "".into(),
            address: "".into(),
            main_address: "".into(),
            main_fee: "".into(),
            utxo_type: UtxoType::Regular,
        }
    }
}

impl UtxoCreator {
    pub fn show(&mut self, app: &mut App, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Create");
            egui::ComboBox::from_id_source("utxo_type")
                .selected_text(format!("{}", self.utxo_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.utxo_type, UtxoType::Regular, "regular");
                    ui.selectable_value(&mut self.utxo_type, UtxoType::Withdrawal, "withdrawal");
                });
            ui.heading("UTXO");
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.monospace("Value:       ");
            ui.add(egui::TextEdit::singleline(&mut self.value));
            ui.monospace("BTC");
        });
        ui.horizontal(|ui| {
            ui.monospace("Address:     ");
            ui.add(egui::TextEdit::singleline(&mut self.address));
            if ui.button("generate").clicked() {
                self.address = app
                    .wallet
                    .get_new_address()
                    .map(|address| format!("{address}"))
                    .unwrap_or("".into());
            }
        });
        if self.utxo_type == UtxoType::Withdrawal {
            ui.horizontal(|ui| {
                ui.monospace("Main Address:");
                ui.add(egui::TextEdit::singleline(&mut self.main_address));
                if ui.button("generate").clicked() {
                    let main_address = app.get_new_main_address().unwrap();
                    self.main_address = format!("{main_address}");
                }
            });
            ui.horizontal(|ui| {
                ui.monospace("Main Fee:    ");
                ui.add(egui::TextEdit::singleline(&mut self.main_fee));
                ui.monospace("BTC");
            });
        }
        ui.horizontal(|ui| {
            match self.utxo_type {
                UtxoType::Regular => {
                    let address: Option<types::Address> = self.address.parse().ok();
                    let value: Option<bitcoin::Amount> =
                        bitcoin::Amount::from_str_in(&self.value, bitcoin::Denomination::Bitcoin)
                            .ok();
                    if ui
                        .add_enabled(
                            address.is_some() && value.is_some(),
                            egui::Button::new("create"),
                        )
                        .clicked()
                    {
                        let utxo = Output {
                            address: address.expect("should not happen"),
                            content: Content::Value(value.expect("should not happen").to_sat()),
                        };
                        app.transaction.outputs.push(utxo);
                    }
                }
                UtxoType::Withdrawal => {
                    let value: Option<bitcoin::Amount> =
                        bitcoin::Amount::from_str_in(&self.value, bitcoin::Denomination::Bitcoin)
                            .ok();
                    let address: Option<types::Address> = self.address.parse().ok();
                    let main_address: Option<bitcoin::Address<bitcoin::address::NetworkUnchecked>> =
                        self.main_address.parse().ok();
                    let main_fee: Option<bitcoin::Amount> = bitcoin::Amount::from_str_in(
                        &self.main_fee,
                        bitcoin::Denomination::Bitcoin,
                    )
                    .ok();
                    if ui
                        .add_enabled(
                            value.is_some()
                                && address.is_some()
                                && main_address.is_some()
                                && main_fee.is_some(),
                            egui::Button::new("create"),
                        )
                        .clicked()
                    {
                        let utxo = Output {
                            address: address.expect("invalid address"),
                            content: Content::Withdrawal {
                                value: value.expect("invalid value").to_sat(),
                                main_address: main_address.expect("invalid main_address"),
                                main_fee: main_fee.expect("invalid main_fee").to_sat(),
                            },
                        };
                        app.transaction.outputs.push(utxo);
                    }
                }
            }
            let num_addresses = app.wallet.get_num_addresses().unwrap();
            ui.label(format!("{num_addresses} addresses generated"));
        });
    }
}
