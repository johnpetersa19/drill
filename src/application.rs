/* application.rs
 *
 * Copyright 2026 John Peter Sa
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::DrillWindow;

const MIN_PREFERENCES_WIDTH: i32 = 520;
const MIN_PREFERENCES_HEIGHT: i32 = 360;
const MAX_PREFERENCES_WIDTH: i32 = 1400;
const MAX_PREFERENCES_HEIGHT: i32 = 1000;

fn database_path_subtitle(path: &str) -> String {
    if path.is_empty() {
        gettext("No local database selected")
    } else {
        path.to_string()
    }
}

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct DrillApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for DrillApplication {
        const NAME: &'static str = "DrillApplication";
        type Type = super::DrillApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for DrillApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<control>q"]);
            obj.set_accels_for_action("app.shortcuts", &["<control>question"]);
        }
    }

    impl ApplicationImpl for DrillApplication {
        fn activate(&self) {
            let application = self.obj();
            let window = application.active_window().unwrap_or_else(|| {
                let window = DrillWindow::new(&*application);
                window.upcast()
            });
            window.present();
        }
    }

    impl GtkApplicationImpl for DrillApplication {}
    impl AdwApplicationImpl for DrillApplication {}
}

glib::wrapper! {
    pub struct DrillApplication(ObjectSubclass<imp::DrillApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl DrillApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .property("resource-base-path", "/io/github/johnpetersa/Drill")
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();

        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();

        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| app.show_preferences())
            .build();

        let shortcuts_action = gio::ActionEntry::builder("shortcuts")
            .activate(move |app: &Self, _, _| app.show_shortcuts())
            .build();

        self.add_action_entries([
            quit_action,
            about_action,
            preferences_action,
            shortcuts_action,
        ]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Drill")
            .application_icon("io.github.johnpetersa.Drill")
            .developer_name("John Peter Sa")
            .version(VERSION)
            .developers(vec!["John Peter Sa"])
            .translator_credits(&gettext("translator-credits"))
            .copyright("\u{a9} 2026 John Peter Sa")
            .build();

        about.present(Some(&window));
    }

    fn show_preferences(&self) {
        let window = self.active_window().unwrap();
        let builder =
            gtk::Builder::from_resource("/io/github/johnpetersa/Drill/preferences-dialog.ui");
        let preferences: adw::PreferencesDialog = builder
            .object("preferences")
            .expect("preferences-dialog.ui must contain an object named 'preferences'");
        let database_group: adw::PreferencesGroup =
            builder.object("database_preferences_group").expect(
                "preferences-dialog.ui must contain an object named 'database_preferences_group'",
            );

        let settings = gio::Settings::new("io.github.johnpetersa.Drill");
        self.restore_preferences_size(&preferences, &settings);
        self.setup_preferences_size_memory(&preferences, &settings);
        self.setup_database_preferences(&window, &database_group, &settings);

        preferences.present(Some(&window));
    }

    fn restore_preferences_size(
        &self,
        preferences: &adw::PreferencesDialog,
        settings: &gio::Settings,
    ) {
        let width = settings
            .int("preferences-width")
            .clamp(MIN_PREFERENCES_WIDTH, MAX_PREFERENCES_WIDTH);
        let height = settings
            .int("preferences-height")
            .clamp(MIN_PREFERENCES_HEIGHT, MAX_PREFERENCES_HEIGHT);

        preferences.set_follows_content_size(false);
        preferences.set_content_width(width);
        preferences.set_content_height(height);
    }

    fn setup_preferences_size_memory(
        &self,
        preferences: &adw::PreferencesDialog,
        settings: &gio::Settings,
    ) {
        let settings_for_width = settings.clone();
        preferences.connect_content_width_notify(move |dialog| {
            let width = dialog
                .content_width()
                .clamp(MIN_PREFERENCES_WIDTH, MAX_PREFERENCES_WIDTH);
            let _ = settings_for_width.set_int("preferences-width", width);
        });

        let settings_for_height = settings.clone();
        preferences.connect_content_height_notify(move |dialog| {
            let height = dialog
                .content_height()
                .clamp(MIN_PREFERENCES_HEIGHT, MAX_PREFERENCES_HEIGHT);
            let _ = settings_for_height.set_int("preferences-height", height);
        });
    }

    fn setup_database_preferences(
        &self,
        window: &gtk::Window,
        database_group: &adw::PreferencesGroup,
        settings: &gio::Settings,
    ) {
        let source_model = gtk::StringList::new(&[
            &gettext("Built-in Cheat database"),
            &gettext("Local JSON database"),
        ]);
        let source_row = adw::ComboRow::builder()
            .title(&gettext("Database source"))
            .subtitle(&gettext("Select where Drill loads file signatures from."))
            .model(&source_model)
            .selected(if settings.string("signature-database-source") == "local" {
                1
            } else {
                0
            })
            .build();

        let path_row = adw::ActionRow::builder()
            .title(&gettext("Local database file"))
            .subtitle(&database_path_subtitle(
                &settings.string("signature-database-path"),
            ))
            .build();
        let choose_button = gtk::Button::with_label(&gettext("Choose JSON"));
        choose_button.set_valign(gtk::Align::Center);
        path_row.add_suffix(&choose_button);
        path_row.set_activatable_widget(Some(&choose_button));
        path_row.set_sensitive(source_row.selected() == 1);

        let settings_for_source = settings.clone();
        let path_row_for_source = path_row.clone();
        source_row.connect_selected_notify(move |row| {
            let is_local = row.selected() == 1;
            let source = if is_local { "local" } else { "builtin" };
            let _ = settings_for_source.set_string("signature-database-source", source);
            path_row_for_source.set_sensitive(is_local);
        });

        let settings_for_file = settings.clone();
        let path_row_for_file = path_row.clone();
        let window_for_file = window.clone();
        choose_button.connect_clicked(move |_| {
            let dialog = gtk::FileDialog::builder()
                .title(&gettext("Choose Signature Database"))
                .accept_label(&gettext("Use Database"))
                .modal(true)
                .build();

            let json_filter = gtk::FileFilter::new();
            json_filter.set_name(Some(&gettext("JSON files")));
            json_filter.add_mime_type("application/json");
            json_filter.add_pattern("*.json");

            let filters = gio::ListStore::new::<gtk::FileFilter>();
            filters.append(&json_filter);
            dialog.set_filters(Some(&filters));

            let settings = settings_for_file.clone();
            let path_row = path_row_for_file.clone();
            dialog.open(
                Some(&window_for_file),
                gio::Cancellable::NONE,
                move |result| {
                    if let Ok(file) = result {
                        if let Some(path) = file.path() {
                            let path = path.to_string_lossy().to_string();
                            let _ = settings.set_string("signature-database-source", "local");
                            let _ = settings.set_string("signature-database-path", &path);
                            path_row.set_subtitle(&database_path_subtitle(&path));
                            path_row.set_sensitive(true);
                        }
                    }
                },
            );
        });

        database_group.add(&source_row);
        database_group.add(&path_row);
    }

    fn show_shortcuts(&self) {
        let window = self.active_window().unwrap();
        // ShortcutsWindow generated from shortcuts-dialog.blp via gresource.
        let builder =
            gtk::Builder::from_resource("/io/github/johnpetersa/Drill/shortcuts-dialog.ui");
        let shortcuts_window: gtk::ShortcutsWindow = builder
            .object("shortcuts")
            .expect("shortcuts-dialog.ui must contain an object named 'shortcuts'");
        shortcuts_window.set_transient_for(Some(window.downcast_ref::<gtk::Window>().unwrap()));
        shortcuts_window.present();
    }
}
