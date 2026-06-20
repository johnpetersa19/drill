/* window.rs
 *
 * Copyright 2026 John Peter Sa
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/org/gnome/Example/window.ui")]
    pub struct DrillWindow {
        #[template_child]
        pub selected_path_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub read_status_dot: TemplateChild<gtk::Box>,

        #[template_child]
        pub read_status_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub current_layer_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub onion_core: TemplateChild<gtk::Box>,

        #[template_child]
        pub onion_ring_2: TemplateChild<gtk::Box>,

        #[template_child]
        pub onion_ring_3: TemplateChild<gtk::Box>,

        #[template_child]
        pub onion_ring_4: TemplateChild<gtk::Box>,

        #[template_child]
        pub onion_ring_5: TemplateChild<gtk::Box>,

        #[template_child]
        pub choose_target_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DrillWindow {
        const NAME: &'static str = "DrillWindow";
        type Type = super::DrillWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DrillWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            obj.load_css();
            obj.setup_window_actions();
            obj.set_read_idle();
        }
    }

    impl WidgetImpl for DrillWindow {}
    impl WindowImpl for DrillWindow {}
    impl ApplicationWindowImpl for DrillWindow {}
    impl AdwApplicationWindowImpl for DrillWindow {}
}

glib::wrapper! {
    pub struct DrillWindow(ObjectSubclass<imp::DrillWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl DrillWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    fn load_css(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource("/org/gnome/Example/style.css");

        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn setup_window_actions(&self) {
        let imp = self.imp();

        imp.choose_target_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.demo_start_reading();
            }
        ));
    }

    fn demo_start_reading(&self) {
        self.set_read_reading();

        glib::timeout_add_seconds_local_once(
            2,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    window.set_read_done();
                }
            ),
        );
    }

    fn clear_read_dot_classes(&self) {
        let imp = self.imp();

        imp.read_status_dot.remove_css_class("read-dot-idle");
        imp.read_status_dot.remove_css_class("read-dot-reading");
        imp.read_status_dot.remove_css_class("read-dot-done");
        imp.read_status_dot.remove_css_class("read-dot-error");
    }

    fn clear_onion_state_classes(&self) {
        let imp = self.imp();

        imp.onion_core.remove_css_class("onion-core-idle");
        imp.onion_core.remove_css_class("onion-core-reading");
        imp.onion_core.remove_css_class("onion-core-done");

        imp.onion_ring_2.remove_css_class("onion-layer-active");
        imp.onion_ring_3.remove_css_class("onion-layer-active");
        imp.onion_ring_4.remove_css_class("onion-layer-active");
        imp.onion_ring_5.remove_css_class("onion-layer-active");

        imp.onion_ring_2.remove_css_class("onion-layer-done");
        imp.onion_ring_3.remove_css_class("onion-layer-done");
        imp.onion_ring_4.remove_css_class("onion-layer-done");
        imp.onion_ring_5.remove_css_class("onion-layer-done");
    }

    fn set_read_idle(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.selected_path_label.set_label("No file or folder chosen");
        imp.read_status_label.set_label("Waiting for file");
        imp.current_layer_label.set_label("Current layer: waiting");

        imp.read_status_dot.add_css_class("read-dot-idle");
        imp.onion_core.add_css_class("onion-core-idle");
    }

    fn set_read_reading(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.selected_path_label
            .set_label("/home/john/project/src/main.rs");
        imp.read_status_label.set_label("Reading file...");
        imp.current_layer_label.set_label("Current layer: file");

        imp.read_status_dot.add_css_class("read-dot-reading");
        imp.onion_core.add_css_class("onion-core-reading");
        imp.onion_ring_2.add_css_class("onion-layer-active");
    }

    fn set_read_done(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.read_status_label.set_label("File read");
        imp.current_layer_label.set_label("Current layer: first level");

        imp.read_status_dot.add_css_class("read-dot-done");
        imp.onion_core.add_css_class("onion-core-done");

        imp.onion_ring_2.add_css_class("onion-layer-done");
        imp.onion_ring_3.add_css_class("onion-layer-active");
    }

    #[allow(dead_code)]
    fn set_read_error(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.read_status_label.set_label("Error reading file");
        imp.current_layer_label.set_label("Current layer: error");

        imp.read_status_dot.add_css_class("read-dot-error");
    }
}
