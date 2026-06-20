/* application.rs
 *
 * Copyright 2026 Unknown
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

use gettextrs::gettext;
use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::DrillWindow;

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
            .property("resource-base-path", "/org/gnome/Example")
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

        self.add_action_entries([quit_action, about_action, preferences_action, shortcuts_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutDialog::builder()
            .application_name("Drill")
            .application_icon("org.gnome.Example")
            .developer_name("Unknown")
            .version(VERSION)
            .developers(vec!["Unknown"])
            .translator_credits(&gettext("translator-credits"))
            .copyright("\u00a9 2026 Unknown")
            .build();

        about.present(Some(&window));
    }

    fn show_preferences(&self) {
        let window = self.active_window().unwrap();
        // TODO: implementar PreferencesDialog (Adw.PreferencesDialog)
        // Placeholder para nao deixar a acao sem resposta.
        let dialog = adw::AlertDialog::builder()
            .heading(&gettext("Preferences"))
            .body(&gettext("Preferences not yet implemented."))
            .build();
        dialog.add_response("ok", &gettext("OK"));
        dialog.set_default_response(Some("ok"));
        dialog.present(Some(&window));
    }

    fn show_shortcuts(&self) {
        let window = self.active_window().unwrap();
        // ShortcutsWindow gerado a partir do shortcuts-dialog.blp via gresource.
        let builder = gtk::Builder::from_resource("/org/gnome/Example/shortcuts-dialog.ui");
        let shortcuts_window: gtk::ShortcutsWindow = builder
            .object("shortcuts")
            .expect("shortcuts-dialog.ui deve conter um objeto chamado 'shortcuts'");
        shortcuts_window.set_transient_for(Some(window.downcast_ref::<gtk::Window>().unwrap()));
        shortcuts_window.present();
    }
}
