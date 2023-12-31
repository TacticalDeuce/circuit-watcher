#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hides the terminal

use eframe::egui;
use egui::{vec2, TextEdit};
use egui_extras::{self, RetainedImage};
use http::{header::AUTHORIZATION, HeaderValue};
use league_client_connector::LeagueClientConnector;
use reqwest::{header, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub struct GUI {
    pick_ban_selection: Arc<AtomicBool>,
    rune_page_selection: Arc<AtomicBool>,
    auto_accept: Arc<AtomicBool>,
    spell_selection: Arc<AtomicBool>,
    pick_text: String,
    ban_text: String,
    text: String,
    champion_picks: Arc<Mutex<Vec<(u32, String)>>>,
    ban_picks: Arc<Mutex<Option<(u32, String)>>>,
    champions: Vec<Champion>,
    gameflow_status: Arc<Mutex<String>>,
    update: Arc<AtomicBool>,
    images: HashMap<String, RetainedImage>,
    selected_image1: Arc<Mutex<Option<String>>>,
    selected_image2: Arc<Mutex<Option<String>>>,
    no_icon_img: RetainedImage,
    assigned_role: Arc<Mutex<Option<String>>>,

    connection_status: Arc<Mutex<Option<String>>>,
    update_status: Arc<Mutex<String>>,
    current_version: Arc<Mutex<String>>,
    asset_name: Arc<Mutex<String>>,
    active_tab: usize,

    update_button_clicked: bool,
    clear_label_timer: Option<std::time::Instant>,
    pick_not_found_label_timer: Option<std::time::Instant>,
    ban_not_found_label_timer: Option<std::time::Instant>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
/// The `Champion` struct is a data structure used for (de)serialization of the `champsions.json` file.
///
/// ### Properties:
/// * `id`: The `id` property is of type `u32`, which stands for "unsigned 32-bit integer". It is used
/// to uniquely identify each instance of the `Champion` struct.
/// * `name`: The `name` property is a string that represents the name of a champion.
struct Champion {
    id: u32,
    name: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Clone)]
/// The `ActionResponseData` struct is a data structure used to represent the response data for a champion select action.
///
/// ### Properties:
/// * `actorCellId`: The `actorCellId` property is of type `i32`, which stands for a 32-bit signed
/// integer. It represents the ID of a summoner in the given champion selection lobby.
/// * `completed`: The "completed" property is a boolean value that indicates whether the action
/// associated with the response data has been completed or not.
/// * `id`: The `id` property is of type `i32`, which stands for a 32-bit signed integer. It is used to
/// uniquely identify an action response data object. It differs from the `actorCellId` by being a unique id tied to the action `r#type`.
/// * `isInProgress`: The `isInProgress` property is a boolean value that indicates whether the action
/// is currently in progress or not.
/// * `r#type`: The property "r#type" is a string that represents the type of action response data. The
/// "r#" prefix is used to escape the reserved keyword "type" in Rust.
struct ActionResponseData {
    actorCellId: i32,
    completed: bool,
    id: i32,
    isInProgress: bool,
    r#type: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug, Clone)]
struct MyTeamData {
    cellId: u32,
    assignedPosition: String,
    spell1Id: u32,
    spell2Id: u32,
}

#[derive(Deserialize, Debug)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize, Debug)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize, Debug)]
struct SummonerSpell {
    key: u32,
    name: String,
}

impl GUI {
    fn new(/*cc: &eframe::CreationContext<'_>*/) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        // Initialize checkbox states
        let pick_ban_selection = Arc::new(AtomicBool::new(false));
        let rune_page_selection = Arc::new(AtomicBool::new(false));
        let auto_accept = Arc::new(AtomicBool::new(false));
        let summoner_spell_selection = Arc::new(AtomicBool::new(false));
        let connection_status = Arc::new(Mutex::new(None));
        let json_data =
            std::fs::read_to_string("./utils/champions.json").expect("Failed to read file");
        let champions: Vec<Champion> =
            serde_json::from_str(&json_data).expect("Failed to parse JSON");

        let mut images: HashMap<String, RetainedImage> = HashMap::new();

        let barrier_img = image_loader("Barrier", include_bytes!("../utils/images/barrier.png"));
        let exhaust_img = image_loader("Exhaust", include_bytes!("../utils/images/exhaust.png"));
        let flash_img = image_loader("Flash", include_bytes!("../utils/images/flash.png"));
        let ghost_img = image_loader("Ghost", include_bytes!("../utils/images/ghost.png"));
        let heal_img = image_loader("Heal", include_bytes!("../utils/images/heal.png"));
        let ignite_img = image_loader("Ignite", include_bytes!("../utils/images/ignite.png"));
        let smite_img = image_loader("Smite", include_bytes!("../utils/images/smite.png"));
        let teleport_img = image_loader("Teleport", include_bytes!("../utils/images/teleport.png"));
        let no_icon_img = image_loader("no_icon", include_bytes!("../utils/images/no_icon.png")).1;

        images.insert(barrier_img.0, barrier_img.1);
        images.insert(exhaust_img.0, exhaust_img.1);
        images.insert(flash_img.0, flash_img.1);
        images.insert(ghost_img.0, ghost_img.1);
        images.insert(heal_img.0, heal_img.1);
        images.insert(ignite_img.0, ignite_img.1);
        images.insert(smite_img.0, smite_img.1);
        images.insert(teleport_img.0, teleport_img.1);

        Self {
            pick_ban_selection,
            rune_page_selection,
            auto_accept,
            pick_text: String::new().to_owned(),
            ban_text: String::new().to_owned(),
            champion_picks: Arc::new(Mutex::new(Vec::new())),
            ban_picks: Arc::new(Mutex::new(None)),
            clear_label_timer: None,
            pick_not_found_label_timer: None,
            ban_not_found_label_timer: None,
            connection_status,
            champions,
            text: String::new().to_owned(),
            gameflow_status: Arc::new(Mutex::new(String::new())),
            update_status: Arc::new(Mutex::new(String::new())),
            current_version: Arc::new(Mutex::new(String::new())),
            update: Arc::new(AtomicBool::new(false)),
            update_button_clicked: false,
            asset_name: Arc::new(Mutex::new("./utils/champions.json".to_owned())), // champions.json will always be in the folder and has a really small size.
            images,
            selected_image1: Arc::new(Mutex::new(None)),
            selected_image2: Arc::new(Mutex::new(None)),
            no_icon_img,
            spell_selection: summoner_spell_selection,
            assigned_role: Arc::new(Mutex::new(None)),
            active_tab: 0,
        }
    }
}

impl eframe::App for GUI {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let pick_ban_selection = self.pick_ban_selection.load(Ordering::SeqCst);
        if let Some(timer) = self.clear_label_timer {
            let elapsed = timer.elapsed();
            if elapsed.as_secs_f32() > 3.0 {
                self.clear_label_timer = None;
            }
        }
        if let Some(timer) = self.pick_not_found_label_timer {
            let elapsed = timer.elapsed();
            if elapsed.as_secs_f32() > 1.5 {
                self.pick_not_found_label_timer = None;
            }
        }
        if let Some(timer) = self.ban_not_found_label_timer {
            let elapsed = timer.elapsed();
            if elapsed.as_secs_f32() > 1.5 {
                self.ban_not_found_label_timer = None;
            }
        }
        let mut champion_picks = self.champion_picks.lock().unwrap();
        let mut ban_picks = self.ban_picks.lock().unwrap();
        let connection_status = self.connection_status.lock().unwrap();
        let gameflow_status = self.gameflow_status.lock().unwrap();
        let mut selected_image1 = self.selected_image1.lock().unwrap();
        let mut selected_image2 = self.selected_image2.lock().unwrap();
        let update_status = self.update_status.lock().unwrap().clone();
        let current_version = self.current_version.lock().unwrap().clone();

        egui::TopBottomPanel::top("top panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let style: egui::Style = (*ui.ctx().style()).clone();
                let new_visuals = style.visuals.light_dark_small_toggle_button(ui);
                if let Some(visuals) = new_visuals {
                    ui.ctx().set_visuals(visuals);
                }

                ui.menu_button("File", |ui| {
                    // TODO: add persistent settings
                    // if ui.button("Save Settings").clicked() {

                    // }

                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });

                if update_status.contains("outdated") {
                    if ui.button("Update").clicked() {
                        self.update_button_clicked = true;
                        self.update.store(true, Ordering::SeqCst);
                    }
                    let asset_name = self.asset_name.lock().unwrap().clone();
                    let asset_size = std::fs::metadata(&asset_name).unwrap().len();

                    if self.update_button_clicked {
                        if asset_size / 1024 > 2000 {
                            egui::Window::new("Updated")
                                .auto_sized()
                                .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, -25.0))
                                .collapsible(false)
                                .movable(false)
                                .show(ctx, |ui| {
                                    ui.label(
                                        "New update has been downloaded successfully to this program's folder.",
                                    );
                                    ui.label("Press the close button to terminate the program.");

                                    if ui.button("Close").clicked() {
                                        frame.close();
                                    }
                            });
                        } else {
                            ui.spinner();
                        }
                    }
                }

                ui.add_space(ui.available_width() - 35.0);

                ui.menu_button("About", |ui| {
                    ui.label("circuit-watcher");
                    ui.label(format!("version {}", current_version));
                    ui.add(egui::Hyperlink::from_label_and_url(
                        "source code",
                        "https://github.com/TacticalDeuce/circuit-watcher",
                    ));
                });
            });
        });

        egui::SidePanel::left("tabs_panel")
            .resizable(false)
            .exact_width(78.0)
            .show(ctx, |ui| {
                let tabs = ["Settings", "Match State"];
                ui.with_layout(
                    egui::Layout::top_down_justified(egui::Align::Center),
                    |ui| {
                        for (idx, label) in tabs.iter().enumerate() {
                            let button = ui.button(*label);

                            if self.active_tab != idx {
                                if button.clicked() {
                                    self.active_tab = idx;
                                }
                            } else {
                                button.highlight();
                            }
                        }
                    },
                );
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                0 => {
                    ui.horizontal(|ui| {
                        if ui.button("Clear Picks/Bans").clicked() {
                            champion_picks.clear();
                            *ban_picks = None;
                            self.clear_label_timer = Some(std::time::Instant::now());
                        }
                        if self.clear_label_timer.is_some() {
                            ui.strong("Picks and bans cleared.");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.menu_image_button(
                            selected_image1
                                .clone()
                                .as_ref()
                                .and_then(|key| self.images.get(key))
                                .map(|img| img.texture_id(ctx))
                                .unwrap_or(self.no_icon_img.texture_id(ctx)),
                            egui::vec2(20.0, 20.0),
                            |ui| {
                                ui.horizontal(|ui| {
                                    for (key, image) in &self.images {
                                        if ui
                                            .add(egui::ImageButton::new(
                                                image.texture_id(ctx),
                                                egui::vec2(17.0, 17.0),
                                            ))
                                            .clicked()
                                        {
                                            if key == &selected_image2.clone().unwrap_or_default() {
                                                let temp = selected_image2.clone();
                                                *selected_image2 = selected_image1.clone();
                                                *selected_image1 = temp;
                                            } else {
                                                *selected_image1 = Some(key.clone());
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                });
                            },
                        );

                        ui.menu_image_button(
                            selected_image2
                                .clone()
                                .as_ref()
                                .and_then(|key| self.images.get(key))
                                .map(|img| img.texture_id(ctx))
                                .unwrap_or(self.no_icon_img.texture_id(ctx)),
                            egui::vec2(20.0, 20.0),
                            |ui| {
                                ui.horizontal(|ui| {
                                    for (key, image) in &self.images {
                                        if ui
                                            .add(egui::ImageButton::new(
                                                image.texture_id(ctx),
                                                egui::vec2(17.0, 17.0),
                                            ))
                                            .clicked()
                                        {
                                            if key == &selected_image1.clone().unwrap_or_default() {
                                                let temp = selected_image2.clone();
                                                *selected_image2 = selected_image1.clone();
                                                *selected_image1 = temp;
                                            } else {
                                                *selected_image2 = Some(key.clone());
                                            }
                                            ui.close_menu();
                                        }
                                    }
                                });
                            },
                        );
                    });

                    ui.horizontal(|ui| {
                        let spell_selection_label = if self.spell_selection.load(Ordering::SeqCst) {
                            "Spell Auto Selection: ON"
                        } else {
                            "Spell Auto Selection: OFF"
                        };

                        if ui
                            .checkbox(
                                &mut self.spell_selection.load(Ordering::SeqCst),
                                spell_selection_label,
                            )
                            .clicked()
                        {
                            let current_state = self.spell_selection.load(Ordering::SeqCst);
                            self.spell_selection.store(!current_state, Ordering::SeqCst);
                        }
                    });

                    if (selected_image1.clone().is_none() || selected_image2.clone().is_none())
                        && self.spell_selection.load(Ordering::SeqCst)
                    {
                        ui.strong("Both summoner spells need to be selected");
                    }

                    ui.horizontal(|ui| {
                        let auto_accept_label = if self.auto_accept.load(Ordering::SeqCst) {
                            "Auto Accept: ON"
                        } else {
                            "Auto Accept: OFF"
                        };

                        if ui
                            .checkbox(
                                &mut self.auto_accept.load(Ordering::SeqCst),
                                auto_accept_label,
                            )
                            .clicked()
                        {
                            let current_state = self.auto_accept.load(Ordering::SeqCst);
                            self.auto_accept.store(!current_state, Ordering::SeqCst);
                        }
                    });

                    // TODO:
                    // ui.horizontal(|ui| {
                    //     let rune_page_label = if self.rune_page_selection.load(Ordering::SeqCst) {
                    //         "Rune Page Change: ON"
                    //     } else {
                    //         "Rune Page Change: OFF"
                    //     };

                    //     if ui
                    //         .checkbox(
                    //             &mut self.rune_page_selection.load(Ordering::SeqCst),
                    //             rune_page_label,
                    //         )
                    //         .clicked()
                    //     {
                    //         let current_state = self.rune_page_selection.load(Ordering::SeqCst);
                    //         self.rune_page_selection
                    //             .store(!current_state, Ordering::SeqCst);
                    //     }
                    // });

                    ui.horizontal(|ui| {
                        let pick_ban_label = if self.pick_ban_selection.load(Ordering::SeqCst) {
                            "Auto-Pick/Ban: ON"
                        } else {
                            "Auto-Pick/Ban: OFF"
                        };

                        if ui
                            .checkbox(
                                &mut self.pick_ban_selection.load(Ordering::SeqCst),
                                pick_ban_label,
                            )
                            .clicked()
                        {
                            let current_state = self.pick_ban_selection.load(Ordering::SeqCst);
                            self.pick_ban_selection
                                .store(!current_state, Ordering::SeqCst);
                        }
                    });

                    ui.vertical(|ui| {
                        if pick_ban_selection {
                            if champion_picks.len() < 2 {
                                ui.label("Enter champions to pick (2 max):");
                                let text_edit_picks = ui.add(
                                    TextEdit::singleline(&mut self.pick_text)
                                        .hint_text("Press enter to skip."),
                                );

                                if !self.pick_text.is_empty() {
                                    let pick_text_cleaned = self
                                        .pick_text
                                        .trim()
                                        .replace(" ", "")
                                        .as_str()
                                        .replace("'", "")
                                        .to_lowercase();

                                    let matching_champions: Vec<String> = self
                                        .champions
                                        .iter()
                                        .filter(|champion| {
                                            champion
                                                .name
                                                .to_lowercase()
                                                .starts_with(&pick_text_cleaned)
                                        })
                                        .map(|champion| champion.name.clone())
                                        .collect();

                                    if !matching_champions.is_empty() {
                                        ui.push_id("pick suggestion", |ui| {
                                            // this is done to ensure no id clash
                                            eframe::egui::ComboBox::from_label("Name Suggestions")
                                                .selected_text(matching_champions[0].clone())
                                                .width(ui.available_width() / 3.0)
                                                .show_ui(ui, |ui| {
                                                    for suggestion in matching_champions {
                                                        if ui
                                                            .selectable_value(
                                                                &mut self.pick_text,
                                                                suggestion.clone(),
                                                                suggestion,
                                                            )
                                                            .clicked()
                                                        {
                                                            text_edit_picks.request_focus();
                                                        }
                                                    }
                                                });
                                        });
                                    }
                                }

                                if text_edit_picks.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    let pick_text_cleaned = self
                                        .pick_text
                                        .trim()
                                        .replace(" ", "")
                                        .as_str()
                                        .replace("'", "")
                                        .to_lowercase();

                                    let matching_champion =
                                        self.champions.iter().find(|champion| {
                                            champion.name.to_lowercase() == pick_text_cleaned
                                        });

                                    if !pick_text_cleaned.is_empty() {
                                        match matching_champion {
                                            Some(champion) => {
                                                if champion_picks
                                                    .contains(&(champion.id, champion.name.clone()))
                                                {
                                                    self.text =
                                                        "Champion has alread been selected."
                                                            .to_string();
                                                    self.pick_not_found_label_timer =
                                                        Some(std::time::Instant::now());
                                                } else {
                                                    champion_picks
                                                        .push((champion.id, champion.name.clone()));
                                                }
                                            }
                                            None => {
                                                self.text =
                                                    "No champion found with the given name."
                                                        .to_string();
                                                self.pick_not_found_label_timer =
                                                    Some(std::time::Instant::now());
                                            }
                                        }
                                    } else {
                                        champion_picks.push((0, "".to_string()));
                                    }
                                    self.pick_text.clear();
                                    text_edit_picks.request_focus();
                                }
                                if self.pick_not_found_label_timer.is_some() {
                                    ui.weak(&self.text);
                                }
                            }

                            if ban_picks.is_none() {
                                ui.label("Enter champion to ban:");
                                let text_edit_bans = ui.add(
                                    TextEdit::singleline(&mut self.ban_text)
                                        .hint_text("Press enter to skip."),
                                );

                                if !self.ban_text.is_empty() {
                                    let ban_text_cleaned = self
                                        .ban_text
                                        .trim()
                                        .replace(" ", "")
                                        .as_str()
                                        .replace("'", "")
                                        .to_lowercase();

                                    let matching_champions: Vec<String> = self
                                        .champions
                                        .iter()
                                        .filter(|champion| {
                                            champion
                                                .name
                                                .to_lowercase()
                                                .starts_with(&ban_text_cleaned)
                                        })
                                        .map(|champion| champion.name.clone())
                                        .collect();

                                    if !matching_champions.is_empty() {
                                        eframe::egui::ComboBox::from_label("Name Suggestions")
                                            .selected_text(matching_champions[0].clone())
                                            .width(ui.available_width() / 3.0)
                                            .show_ui(ui, |ui| {
                                                for suggestion in matching_champions {
                                                    if ui
                                                        .selectable_value(
                                                            &mut self.ban_text,
                                                            suggestion.clone(),
                                                            suggestion,
                                                        )
                                                        .clicked()
                                                    {
                                                        text_edit_bans.request_focus();
                                                    }
                                                }
                                            });
                                    }
                                }

                                if text_edit_bans.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    let ban_text_cleaned = self
                                        .ban_text
                                        .trim()
                                        .replace(" ", "")
                                        .as_str()
                                        .replace("'", "")
                                        .to_lowercase();

                                    let matching_champion =
                                        self.champions.iter().find(|champion| {
                                            champion.name.to_lowercase() == ban_text_cleaned
                                        });

                                    if !ban_text_cleaned.is_empty() {
                                        match matching_champion {
                                            Some(champion) => {
                                                if champion_picks
                                                    .contains(&(champion.id, champion.name.clone()))
                                                {
                                                    self.text =
                                                        "Champion has alread been selected."
                                                            .to_string();
                                                    self.ban_not_found_label_timer =
                                                        Some(std::time::Instant::now());
                                                } else {
                                                    *ban_picks =
                                                        Some((champion.id, champion.name.clone()));
                                                }
                                            }
                                            None => {
                                                self.text =
                                                    "No champion found with the given name."
                                                        .to_string();
                                                self.ban_not_found_label_timer =
                                                    Some(std::time::Instant::now());
                                            }
                                        }
                                    } else {
                                        *ban_picks = Some((
                                            0,
                                            self.ban_text
                                                .trim()
                                                .replace(" ", "")
                                                .as_str()
                                                .replace("'", "")
                                                .to_string()
                                                .to_lowercase(),
                                        ));
                                    }
                                    self.ban_text.clear();
                                    text_edit_bans.request_focus();
                                }
                                if self.ban_not_found_label_timer.is_some() {
                                    ui.weak(&self.text);
                                }
                            }
                        }
                        if pick_ban_selection {
                            if champion_picks.len() == 2
                                && champion_picks.get(0).unwrap().1.is_empty()
                                && ban_picks.is_some()
                                && ban_picks.as_ref().unwrap().1.is_empty()
                                && champion_picks.get(1).unwrap().1.is_empty()
                            {
                                champion_picks.clear();
                                *ban_picks = None;
                                self.pick_ban_selection.store(false, Ordering::SeqCst);
                            }
                            if champion_picks.len() != 0 {
                                ui.strong("Picks:");
                                for (id, name) in &*champion_picks {
                                    if !name.is_empty() {
                                        ui.label(format!("ID:{id} Name:\"{name}\""));
                                    } else {
                                        ui.label("None");
                                    }
                                }
                            }
                            if ban_picks.is_some() {
                                ui.strong("Ban:");
                                if ban_picks.as_ref().unwrap().1.is_empty() {
                                    ui.label("None");
                                } else {
                                    ui.label(format!(
                                        "ID:{} Name:\"{}\"",
                                        &ban_picks.as_ref().unwrap().0,
                                        &ban_picks.as_ref().unwrap().1
                                    ));
                                }
                            }
                        }
                    });
                }
                1 => {
                    ui.heading(format!("{}", gameflow_status.clone()));
                    if let Some(assigned_role) = self.assigned_role.lock().unwrap().clone() {
                        ui.label(format!("Role: {}", assigned_role));
                    }
                }
                2 => {}
                _ => unreachable!(),
            }

            ui.vertical_centered_justified(|ui| {
                ui.add_space(ui.available_size().y - ui.spacing().item_spacing.y * 11.0);
                ui.weak(update_status);
                if let Some(status) = connection_status.clone() {
                    ui.weak(status.clone());
                }
            });
        });

        ctx.request_repaint_after(tokio::time::Duration::from_millis(500));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        std::process::exit(0);
    }
}

async fn update_checker(update_status: Arc<Mutex<String>>) -> Result<String, Box<dyn Error>> {
    let repo_owner = "tacticaldeuce";
    let repo_name = "circuit-watcher";
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        repo_owner, repo_name
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header(
            "User-Agent",
            format!("CircuitWatcher/{} (Rust)", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await?;
    let json = response.json::<serde_json::Value>().await?;

    let latest_tag = json["tag_name"].as_str().unwrap();

    let current_version = env!("CARGO_PKG_VERSION");

    let mut update_status = update_status.lock().unwrap();

    if !latest_tag.contains(current_version) {
        *update_status =
            format!("Program is outdated the latest version is {}", latest_tag).to_owned();
    } else {
        *update_status = "Program is up to date.".to_owned();
    }

    Ok(current_version.to_owned())
}

fn hide_console_window() {
    use std::ptr;
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_HIDE};

    let window = unsafe { GetConsoleWindow() };
    // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showwindow
    if window != ptr::null_mut() {
        unsafe {
            ShowWindow(window, SW_HIDE);
        }
    }
}

fn image_loader(img_name: &str, img_bytes: &[u8]) -> (String, RetainedImage) {
    (
        img_name.to_string(),
        RetainedImage::from_image_bytes(img_name, img_bytes).unwrap(),
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let options = eframe::NativeOptions {
        // icon_data: None,
        min_window_size: Some(vec2(330.0, 320.0)),
        initial_window_size: Some(egui::vec2(500.0, 400.0)),
        ..Default::default()
    };

    let app = GUI::new();

    let champion_picks_clone = Arc::clone(&app.champion_picks);
    let ban_picks_clone = Arc::clone(&app.ban_picks);
    let connection_status = Arc::clone(&app.connection_status);
    let connection_status_clone = Arc::clone(&app.connection_status);
    let gameflow_status = Arc::clone(&app.gameflow_status);
    let pick_ban_selection_clone = Arc::clone(&app.pick_ban_selection);
    let rune_page_change_clone = Arc::clone(&app.rune_page_selection);
    let auto_accept_clone = Arc::clone(&app.auto_accept);
    let update_status_clone = Arc::clone(&app.update_status);
    let current_version_clone = Arc::clone(&app.current_version);
    let update_clone = Arc::clone(&app.update);
    let asset_name_clone = Arc::clone(&app.asset_name);
    let selected_image1_clone = Arc::clone(&app.selected_image1);
    let selected_image2_clone = Arc::clone(&app.selected_image2);
    let spell_selection_clone = Arc::clone(&app.spell_selection);
    let assigned_role_clone = Arc::clone(&app.assigned_role);

    tokio::spawn(async move {
        loop {
            hide_console_window();
            let update = update_clone.load(Ordering::SeqCst);
            let asset_name = Arc::clone(&asset_name_clone);

            if update {
                let client = reqwest::Client::new();

                let owner = "tacticaldeuce";
                let repo = "circuit-watcher";

                let url = format!(
                    "https://api.github.com/repos/{}/{}/releases/latest",
                    owner, repo
                );
                let response = client
                    .get(&url)
                    .header(
                        "User-Agent",
                        format!("CircuitWatcher/{} (Rust)", env!("CARGO_PKG_VERSION")),
                    )
                    .send()
                    .await
                    .unwrap();
                let status = response.status();
                let body: serde_json::Value = response.json().await.unwrap();
                let release: Release = serde_json::from_value(body).unwrap();

                if status.is_success() {
                    for asset in release.assets {
                        let asset_url = asset.browser_download_url.clone();

                        let response = client.get(&asset_url).send().await.unwrap();

                        let file_name = asset.name.clone();
                        let mut file = std::fs::File::create(&file_name).unwrap();
                        let contents = response.bytes().await.unwrap();

                        file.write_all(&contents).unwrap();

                        *asset_name.lock().unwrap() = asset.name.clone();
                        update_clone.store(false, Ordering::SeqCst);
                    }
                }
            }
            match LeagueClientConnector::parse_raw_info() {
                Ok(lockfile) => {
                    let mut status = connection_status.lock().unwrap();
                    *status = Some(format!(
                        "Connected to LeagueClient on https://127.0.0.1:{}",
                        lockfile.port
                    ));
                }
                Err(_) => {
                    let mut status = connection_status.lock().unwrap();
                    *status = Some("LeagueClient not found, may be closed.".to_owned());
                }
            }
        }
    });

    tokio::spawn(async move {
        let status = connection_status_clone.lock().unwrap().clone();
        let current_version_clone = Arc::clone(&current_version_clone);

        *current_version_clone.lock().unwrap() = update_checker(update_status_clone).await.unwrap();

        // Both of this while loops are to ensure there is a viable connection to the League Client
        while status.is_none() {
            if connection_status_clone.lock().unwrap().clone().is_some() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        }
        while connection_status_clone
            .lock()
            .unwrap()
            .clone()
            .as_ref()
            .unwrap()
            .contains("LeagueClient not found, may be closed.")
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

        let mut lc_info = LeagueClientConnector::parse_raw_info().unwrap();
        let mut auth_header =
            HeaderValue::from_str(format!("Basic {}", lc_info.b64_auth).as_str()).unwrap();
        let cert =
            reqwest::Certificate::from_pem(include_bytes!("../utils/riotgames.pem")).unwrap();
        let mut headers = header::HeaderMap::new();

        headers.insert(AUTHORIZATION, auth_header.clone());
        let mut rest_client = ClientBuilder::new()
            .add_root_certificate(cert.clone())
            .default_headers(headers)
            .build()
            .unwrap();

        let spells_data =
            std::fs::read_to_string("./utils/summoner_spells.json").expect("Failed to read file");
        let summoner_spells: Vec<SummonerSpell> =
            serde_json::from_str(&spells_data).expect("Failed to parse JSON");

        let mut locked_champ = false;
        loop {
            if connection_status_clone
                .lock()
                .unwrap()
                .clone()
                .as_ref()
                .unwrap()
                .contains("LeagueClient not found, may be closed.")
            {
                match LeagueClientConnector::parse_raw_info() {
                    Ok(riotlockfile) => {
                        lc_info = riotlockfile;
                        auth_header =
                            HeaderValue::from_str(format!("Basic {}", lc_info.b64_auth).as_str())
                                .unwrap();
                        headers = header::HeaderMap::new();

                        headers.insert(AUTHORIZATION, auth_header.clone());
                        rest_client = ClientBuilder::new()
                            .add_root_certificate(cert.clone())
                            .default_headers(headers)
                            .build()
                            .unwrap();

                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }
                    Err(_) => {
                        continue;
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }

            let champion_picks = champion_picks_clone.lock().unwrap().clone();
            let ban_picks = ban_picks_clone.lock().unwrap().clone();
            let gameflow_status_clone = Arc::clone(&gameflow_status);
            let pick_ban_selection = pick_ban_selection_clone.load(Ordering::SeqCst);
            let rune_change = rune_page_change_clone.load(Ordering::SeqCst);
            let auto_accept = auto_accept_clone.load(Ordering::SeqCst);
            let spell1 = Arc::clone(&selected_image1_clone);
            let spell2 = Arc::clone(&selected_image2_clone);
            let spell_selection = spell_selection_clone.load(Ordering::SeqCst);
            let assigned_position = Arc::clone(&assigned_role_clone);

            let gameflow: serde_json::Value = rest_client
                .get(format!(
                    "https://127.0.0.1:{}/lol-gameflow/v1/session",
                    lc_info.port
                ))
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let phase = gameflow["phase"].as_str();

            match phase {
                Some("Matchmaking") => {
                    *assigned_position.lock().unwrap() = None;
                    *gameflow_status_clone.lock().unwrap() = "Looking for a match".to_owned();
                    locked_champ = false;
                }
                Some("Lobby") => {
                    *assigned_position.lock().unwrap() = None;
                    *gameflow_status_clone.lock().unwrap() = "In Lobby".to_owned();
                }
                Some("ReadyCheck") => {
                    if auto_accept {
                        *gameflow_status_clone.lock().unwrap() = "Accepting match".to_owned();
                        rest_client
                            .post(format!(
                                "https://127.0.0.1:{}/lol-matchmaking/v1/ready-check/accept",
                                lc_info.port
                            ))
                            .send()
                            .await
                            .unwrap();
                    }
                    *gameflow_status_clone.lock().unwrap() = "Match Found".to_owned();
                }
                Some("ChampSelect") => {
                    let current_champ_select: serde_json::Value = rest_client
                        .get(format!(
                            "https://127.0.0.1:{}/lol-champ-select/v1/session",
                            lc_info.port
                        ))
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();

                    let team_data_response: Vec<MyTeamData> =
                        serde_json::from_value(current_champ_select["myTeam"].clone()).unwrap();
                    let filtered_team_data: Vec<MyTeamData> = team_data_response
                        .iter()
                        .filter(|data| data.cellId == current_champ_select["localPlayerCellId"])
                        .take(1)
                        .cloned() // Limit to a maximum of 2 matches
                        .collect();
                    let extracted_team_data: (u32, u32, String) = filtered_team_data
                        .iter()
                        .map(|data| (data.spell1Id, data.spell2Id, data.assignedPosition.clone()))
                        .next()
                        .unwrap();

                    *assigned_position.lock().unwrap() = Some(extracted_team_data.clone().2);
                    if spell_selection {
                        let spell1_clone = selected_image1_clone.lock().unwrap().clone();
                        let spell2_clone = selected_image2_clone.lock().unwrap().clone();

                        if spell1_clone.is_some() && spell2_clone.is_some() {
                            if extracted_team_data.2.contains("jungle") {
                                if spell1_clone.clone().unwrap() != "Smite".to_string()
                                    && spell2_clone.clone().unwrap() != "Smite".to_string()
                                {
                                    if extracted_team_data.0 == 4
                                    /*Flash*/
                                    {
                                        *spell1.lock().unwrap() = Some("Flash".to_owned());
                                        *spell2.lock().unwrap() = Some("Smite".to_owned());
                                        continue;
                                    }
                                    if extracted_team_data.0 == 6
                                    /*Ghost*/
                                    {
                                        *spell1.lock().unwrap() = Some("Ghost".to_owned());
                                        *spell2.lock().unwrap() = Some("Smite".to_owned());
                                        continue;
                                    }
                                    if extracted_team_data.1 == 4 {
                                        *spell1.lock().unwrap() = Some("Smite".to_owned());
                                        *spell2.lock().unwrap() = Some("Flash".to_owned());
                                        continue;
                                    }
                                    if extracted_team_data.1 == 6 {
                                        *spell1.lock().unwrap() = Some("Smite".to_owned());
                                        *spell2.lock().unwrap() = Some("Ghost".to_owned());
                                        continue;
                                    }
                                    *spell1.lock().unwrap() = Some("Smite".to_owned());
                                    continue;
                                }
                            }
                            let spell1_info = summoner_spells
                                .iter()
                                .find(|spell| spell.name == spell1_clone.clone().unwrap())
                                .unwrap();
                            let spell2_info = summoner_spells
                                .iter()
                                .find(|spell| spell.name == spell2_clone.clone().unwrap())
                                .unwrap();

                            let body = serde_json::json!({
                                    "spell1Id": spell1_info.key,
                                    "spell2Id": spell2_info.key
                            });

                            rest_client
                                .patch(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/session/my-selection",
                                    lc_info.port
                                ))
                                .json(&body)
                                .send()
                                .await
                                .unwrap();
                        }
                    }

                    if !pick_ban_selection {
                        *gameflow_status_clone.lock().unwrap() = "Champion Selection".to_owned();
                        continue;
                    }

                    *gameflow_status_clone.lock().unwrap() =
                        "Champion Selection with Auto-pick/ban ON".to_owned();

                    if champion_picks.len() == 0 && ban_picks.is_none() {
                        continue;
                    }

                    let current_champ_select: serde_json::Value = rest_client
                        .get(format!(
                            "https://127.0.0.1:{}/lol-champ-select/v1/session",
                            lc_info.port
                        ))
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .unwrap();

                    let action_response: Vec<Vec<ActionResponseData>> =
                        serde_json::from_value(current_champ_select["actions"].clone()).unwrap();
                    let filtered_action_data: Vec<ActionResponseData> = action_response
                        .iter()
                        .flatten()
                        .filter(|data| {
                            data.actorCellId == current_champ_select["localPlayerCellId"]
                        })
                        .take(2) // Limit to a maximum of 2 matches
                        .cloned()
                        .collect();
                    let extracted_action_data: Vec<(i32, bool, String, bool)> =
                        filtered_action_data
                            .iter()
                            .map(|data| {
                                (
                                    data.id,
                                    data.isInProgress,
                                    data.r#type.clone(),
                                    data.completed,
                                )
                            })
                            .collect();

                    let (ban_id, ban_is_in_progress, _type1, ban_completed) = extracted_action_data
                        .get(0)
                        .cloned()
                        .unwrap_or((0, false, "".to_string(), false));
                    let (pick_id, pick_is_in_progress, _type2, pick_completed) =
                        extracted_action_data.get(1).cloned().unwrap_or((
                            0,
                            false,
                            "".to_string(),
                            false,
                        ));

                    if ban_picks.is_some() {
                        if !ban_picks.as_ref().unwrap().1.is_empty() {
                            let ban_body = serde_json::json!({
                                    "actorCellId": current_champ_select["localPlayerCellId"],
                                    "championId": &ban_picks.as_ref().unwrap().0,
                                    "completed": true,
                                    "id": &ban_id,
                                    "isAllyAction": true,
                                    "type": "ban"
                            });
                            let ban_champ_info: serde_json::Value = rest_client
                                .get(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/grid-champions/{}",
                                    lc_info.port,
                                    &ban_picks.as_ref().unwrap().0
                                ))
                                .send()
                                .await
                                .unwrap()
                                .json()
                                .await
                                .unwrap();

                            if ban_is_in_progress
                                && !ban_completed
                                && ban_champ_info["selectionStatus"]["pickedByOtherOrBanned"]
                                    != true
                                && current_champ_select["timer"]["phase"] != "PLANNING"
                            {
                                rest_client
                                    .patch(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/session/actions/{}",
                                    lc_info.port, ban_id
                                ))
                                    .json(&ban_body)
                                    .send()
                                    .await
                                    .unwrap();
                                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                            }
                        }
                    }

                    if champion_picks.len() != 0 {
                        if champion_picks.get(0).unwrap().1.is_empty()
                            && champion_picks.get(1).unwrap().1.is_empty()
                        {
                            continue;
                        }
                        if !champion_picks.get(0).unwrap().1.is_empty() {
                            let pick_champ_info: serde_json::Value = rest_client
                                .get(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/grid-champions/{}",
                                    lc_info.port,
                                    champion_picks.get(0).unwrap().0
                                ))
                                .send()
                                .await
                                .unwrap()
                                .json()
                                .await
                                .unwrap();

                            let pick_body = serde_json::json!({
                                    "actorCellId": current_champ_select["localPlayerCellId"],
                                    "championId": champion_picks.get(0).unwrap().0,
                                    "completed": true,
                                    "id": &pick_id,
                                    "isAllyAction": true,
                                    "type": "pick"
                            });

                            if !pick_is_in_progress
                                && pick_completed
                                && !ban_is_in_progress
                                && ban_completed
                                || current_champ_select["timer"]["phase"] == "PLANNING"
                            {
                                continue;
                            }

                            if !pick_is_in_progress {
                                continue;
                            }
                            if pick_champ_info["selectionStatus"]["pickedByOtherOrBanned"] != true {
                                if pick_is_in_progress
                                    && !pick_completed
                                    && !ban_is_in_progress
                                    && ban_completed
                                    && pick_champ_info["selectionStatus"]["pickedByOtherOrBanned"]
                                        != true
                                    && !locked_champ
                                {
                                    if rune_change {
                                        // TODO:
                                    }
                                    rest_client
                                        .patch(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/session/actions/{}",
                                    lc_info.port, pick_id
                                ))
                                        .json(&pick_body)
                                        .send()
                                        .await
                                        .unwrap();
                                    locked_champ = true;
                                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                }
                            }
                        }

                        if champion_picks.len() == 1 {
                            continue;
                        }

                        if !champion_picks.get(1).unwrap().1.is_empty() {
                            let pick_champ_info: serde_json::Value = rest_client
                                .get(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/grid-champions/{}",
                                    lc_info.port,
                                    champion_picks.get(1).unwrap().0
                                ))
                                .send()
                                .await
                                .unwrap()
                                .json()
                                .await
                                .unwrap();

                            let pick_body = serde_json::json!({
                                    "actorCellId": current_champ_select["localPlayerCellId"],
                                    "championId": champion_picks.get(1).unwrap().0,
                                    "completed": true,
                                    "id": &pick_id,
                                    "isAllyAction": true,
                                    "type": "pick"
                            });

                            if !pick_is_in_progress
                                && pick_completed
                                && !ban_is_in_progress
                                && ban_completed
                                || current_champ_select["timer"]["phase"] == "PLANNING"
                            {
                                continue;
                            }

                            if !pick_is_in_progress {
                                continue;
                            }
                            if pick_champ_info["selectionStatus"]["pickedByOtherOrBanned"] != true {
                                if pick_is_in_progress
                                    && !pick_completed
                                    && !ban_is_in_progress
                                    && ban_completed
                                    && pick_champ_info["selectionStatus"]["pickedByOtherOrBanned"]
                                        != true
                                    && !locked_champ
                                {
                                    if rune_change {
                                        // TODO:
                                    }
                                    rest_client
                                        .patch(format!(
                                    "https://127.0.0.1:{}/lol-champ-select/v1/session/actions/{}",
                                    lc_info.port, pick_id
                                ))
                                        .json(&pick_body)
                                        .send()
                                        .await
                                        .unwrap();
                                    locked_champ = true;
                                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                                }
                            }
                        }
                    }
                }
                Some("InProgress") => {
                    *gameflow_status_clone.lock().unwrap() = "Game in progress...".to_owned();
                    tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
                }
                Some("WaitingForStats") => {
                    *gameflow_status_clone.lock().unwrap() = "Waiting for Stats".to_owned();
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Some("PreEndOfGame") => {
                    *gameflow_status_clone.lock().unwrap() = "Game in progress...".to_owned();
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
                Some("EndOfGame") => {
                    *assigned_position.lock().unwrap() = None;
                    *gameflow_status_clone.lock().unwrap() = "Game Ending...".to_owned();
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
                Some(unimplemented_phase) => {
                    *assigned_position.lock().unwrap() = None;
                    *gameflow_status_clone.lock().unwrap() =
                        format!("Unimplemented Phase: {}", unimplemented_phase).to_owned();
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
                None => {
                    *gameflow_status_clone.lock().unwrap() = "Idling...".to_owned();
                }
            }
        }
    });

    eframe::run_native("Circuit Watcher", options, Box::new(|_cc| Box::new(app)))?;

    Ok(())
}
