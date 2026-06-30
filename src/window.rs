/* window.rs
 *
 * Copyright 2026 John Peter Sa
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use crate::icon_catalog;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::prelude::*;
use gtk::{gio, glib};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};

/// How many onion rings are visible at once.
const VISIBLE_LAYERS: usize = 10;

#[derive(Clone)]
struct TreeItem {
    branch: &'static str,
    icon: &'static str,
    name: Cow<'static, str>,
}

#[derive(Clone)]
struct LayerSpec {
    label: String,
    current_label: String,
    detail: String,
    state: LayerState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LayerState {
    Idle,
    Active,
    Done,
}

fn empty_tree_items() -> [TreeItem; 1] {
    [TreeItem {
        branch: "",
        icon: "dialog-information-symbolic",
        name: Cow::Owned(gettext("No project tree yet.")),
    }]
}

const READING_TREE_ITEMS: &[TreeItem] = &[TreeItem {
    branch: "└──",
    icon: "text-x-generic-symbolic",
    name: Cow::Borrowed("/home/john/project/src/main.rs"),
}];

const PROJECT_TREE_ITEMS: &[TreeItem] = &[
    TreeItem {
        branch: "├──",
        icon: "folder-symbolic",
        name: Cow::Borrowed("builddir"),
    },
    TreeItem {
        branch: "│   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("build.ninja"),
    },
    TreeItem {
        branch: "│   ├──",
        icon: "folder-symbolic",
        name: Cow::Borrowed("cargo-home"),
    },
    TreeItem {
        branch: "│   │   └──",
        icon: "folder-symbolic",
        name: Cow::Borrowed("registry"),
    },
    TreeItem {
        branch: "│   │       └──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("CACHEDIR.TAG"),
    },
    TreeItem {
        branch: "│   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("compile_commands.json"),
    },
    TreeItem {
        branch: "│   ├──",
        icon: "folder-symbolic",
        name: Cow::Borrowed("data"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "folder-symbolic",
        name: Cow::Borrowed("icons"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("io.github.johnpetersa.Drill.desktop"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("io.github.johnpetersa.Drill.metainfo.xml"),
    },
    TreeItem {
        branch: "│   │   └──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("io.github.johnpetersa.Drill.service"),
    },
    TreeItem {
        branch: "│   ├──",
        icon: "folder-symbolic",
        name: Cow::Borrowed("meson-info"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-benchmarks.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-buildoptions.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-buildsystem_files.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-compilers.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-dependencies.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-installed.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-install_plan.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-machines.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-projectinfo.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-targets.json"),
    },
    TreeItem {
        branch: "│   │   ├──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("intro-tests.json"),
    },
    TreeItem {
        branch: "│   │   └──",
        icon: "text-x-generic-symbolic",
        name: Cow::Borrowed("meson-info.json"),
    },
];

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/io/github/johnpetersa/Drill/window.ui")]
    pub struct DrillWindow {
        #[template_child]
        pub read_status_dot: TemplateChild<gtk::Box>,

        #[template_child]
        pub read_status_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub tree_rows_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub tree_summary_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub current_layer_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub onion_layer_detail_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub onion_layers_fixed: TemplateChild<gtk::Fixed>,

        #[template_child]
        pub onion_core: TemplateChild<gtk::Box>,

        #[template_child]
        pub zoom_in_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub zoom_out_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub zoom_page_label: TemplateChild<gtk::Label>,

        /// Full list of layers from the last call to `set_onion_layers_full`.
        pub all_layers: RefCell<Vec<LayerSpec>>,

        /// Index of the first visible layer in `all_layers`.
        pub layer_offset: Cell<usize>,
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
            obj.setup_zoom_buttons();
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
        provider.load_from_resource("/io/github/johnpetersa/Drill/style.css");

        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }

    fn setup_window_actions(&self) {
        let choose_target_action = gio::ActionEntry::builder("choose-target")
            .activate(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _| {
                    window.demo_start_reading();
                }
            ))
            .build();

        self.add_action_entries([choose_target_action]);
    }

    fn setup_zoom_buttons(&self) {
        let imp = self.imp();

        imp.zoom_in_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                let imp = window.imp();
                let total = imp.all_layers.borrow().len();
                let offset = imp.layer_offset.get();
                let new_offset = (offset + 1).min(total.saturating_sub(VISIBLE_LAYERS));
                imp.layer_offset.set(new_offset);
                window.refresh_onion_viewport();
            }
        ));

        imp.zoom_out_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                let imp = window.imp();
                let offset = imp.layer_offset.get();
                if offset > 0 {
                    imp.layer_offset.set(offset - 1);
                    window.refresh_onion_viewport();
                }
            }
        ));
    }

    /// Store the full layer list, reset offset to 0, render the first window.
    fn set_onion_layers_full(&self, layers: Vec<LayerSpec>) {
        let imp = self.imp();
        imp.all_layers.replace(layers);
        imp.layer_offset.set(0);
        self.refresh_onion_viewport();
    }

    /// Re-render rings using the current offset and update zoom button sensitivity.
    fn refresh_onion_viewport(&self) {
        let imp = self.imp();
        let all = imp.all_layers.borrow();
        let total = all.len();
        let offset = imp.layer_offset.get();

        let end = (offset + VISIBLE_LAYERS).min(total);
        let visible: Vec<LayerSpec> = all[offset..end].to_vec();

        drop(all); // release borrow before calling set_onion_rings
        self.set_onion_rings(&visible);

        let imp = self.imp();
        let total = imp.all_layers.borrow().len();
        let offset = imp.layer_offset.get();

        imp.zoom_out_button.set_sensitive(offset > 0);
        imp.zoom_in_button
            .set_sensitive(offset + VISIBLE_LAYERS < total);

        if total == 0 {
            imp.zoom_page_label.set_label("–");
        } else {
            let first = offset + 1;
            let last = (offset + VISIBLE_LAYERS).min(total);
            imp.zoom_page_label
                .set_label(&format!("{first}–{last} / {total}"));
        }
    }

    fn clear_onion_layers(&self) {
        let imp = self.imp();
        while let Some(child) = imp.onion_layers_fixed.first_child() {
            imp.onion_layers_fixed.remove(&child);
        }
    }

    /// Low-level: draw exactly the rings in `layers` (already sliced).
    fn set_onion_rings(&self, layers: &[LayerSpec]) {
        self.clear_onion_layers();

        if layers.is_empty() {
            return;
        }

        let imp = self.imp();
        let outer_size = 320.0_f64;
        let inner_size = 88.0_f64;
        let step = if layers.len() > 1 {
            (outer_size - inner_size) / (layers.len() as f64 - 1.0)
        } else {
            0.0
        };

        for (index, layer) in layers.iter().enumerate() {
            let size = (outer_size - step * index as f64).round().max(inner_size) as i32;
            let ring = gtk::Box::new(gtk::Orientation::Vertical, 0);
            ring.set_halign(gtk::Align::Center);
            ring.set_valign(gtk::Align::Center);
            ring.set_width_request(size);
            ring.set_height_request(size);
            ring.add_css_class("onion-ring");

            match layer.state {
                LayerState::Active => ring.add_css_class("onion-layer-active"),
                LayerState::Done => ring.add_css_class("onion-layer-done"),
                LayerState::Idle => {}
            }

            ring.set_tooltip_text(Some(layer.label.as_str()));

            let click = gtk::GestureClick::new();
            let window = self.downgrade();
            let current_label = layer.current_label.clone();
            let detail = layer.detail.clone();
            click.connect_pressed(move |_, _, _, _| {
                if let Some(window) = window.upgrade() {
                    window.show_onion_layer(&current_label, &detail);
                }
            });
            ring.add_controller(click);

            let offset = ((340 - size) / 2) as f64;
            imp.onion_layers_fixed.put(&ring, offset, offset);
        }
    }

    fn show_onion_layer(&self, current_label: &str, detail: &str) {
        let imp = self.imp();
        imp.current_layer_label.set_label(current_label);
        imp.onion_layer_detail_label.set_label(detail);
    }

    fn set_tree_items(&self, items: &[TreeItem]) {
        let imp = self.imp();

        while let Some(child) = imp.tree_rows_box.first_child() {
            imp.tree_rows_box.remove(&child);
        }

        for item in items {
            let (depth, connector) = tree_branch_parts(item.branch);
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 3);
            row.set_hexpand(true);
            row.add_css_class("tree-row");

            let indent = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            indent.set_width_request(depth * 14);
            indent.add_css_class("tree-indent");
            row.append(&indent);

            let branch = gtk::Label::new(Some(connector));
            branch.set_xalign(0.0);
            branch.set_width_request(18);
            branch.add_css_class("tree-branch");
            row.append(&branch);

            let icon =
                gtk::Image::from_icon_name(icon_catalog::for_path(item.name.as_ref(), item.icon));
            icon.set_pixel_size(16);
            icon.add_css_class("tree-icon");
            row.append(&icon);

            let name = gtk::Label::new(Some(item.name.as_ref()));
            name.set_hexpand(true);
            name.set_width_chars(1);
            name.set_xalign(0.0);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            name.add_css_class("tree-name");
            row.append(&name);

            imp.tree_rows_box.append(&row);
        }
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
    }

    fn reset_zoom(&self) {
        let imp = self.imp();
        imp.all_layers.replace(vec![]);
        imp.layer_offset.set(0);
        imp.zoom_in_button.set_sensitive(false);
        imp.zoom_out_button.set_sensitive(false);
        imp.zoom_page_label.set_label("–");
        self.clear_onion_layers();
    }

    fn set_read_idle(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();
        self.reset_zoom();

        imp.read_status_label
            .set_label(&gettext("Waiting for file"));
        let items = empty_tree_items();
        self.set_tree_items(&items);
        imp.tree_summary_label
            .set_label(&gettext("Waiting for analysis."));
        imp.current_layer_label
            .set_label(&gettext("Current layer: waiting"));
        imp.onion_layer_detail_label
            .set_label(&gettext("Select a layer to inspect it."));

        imp.read_status_dot.add_css_class("read-dot-idle");
        imp.onion_core.add_css_class("onion-core-idle");
    }

    fn set_read_reading(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.read_status_label.set_label(&gettext("Reading file..."));
        self.set_tree_items(READING_TREE_ITEMS);
        self.set_onion_layers_full(demo_layers(
            1,
            LayerState::Active,
            gettext("Layer 1"),
            gettext("Current layer: file"),
            gettext("Layer 1: file under reading."),
        ));
        imp.tree_summary_label
            .set_label(&gettext("Building the first tree level..."));
        imp.current_layer_label
            .set_label(&gettext("Current layer: file"));
        imp.onion_layer_detail_label
            .set_label(&gettext("Layer 1: file under reading."));

        imp.read_status_dot.add_css_class("read-dot-reading");
        imp.onion_core.add_css_class("onion-core-reading");
    }

    fn set_read_done(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.read_status_label.set_label(&gettext("File read"));
        self.set_tree_items(PROJECT_TREE_ITEMS);
        let layer_count = PROJECT_TREE_ITEMS.len().max(5);
        self.set_onion_layers_full(demo_layers(
            layer_count,
            LayerState::Done,
            gettext("Core"),
            gettext("Current layer: first level"),
            gettext("Layer 3: first data detected during reading."),
        ));
        imp.tree_summary_label.set_label(&gettext(
            "Project tree ready: build files, generated metadata and resources.",
        ));
        imp.current_layer_label
            .set_label(&gettext("Current layer: first level"));
        imp.onion_layer_detail_label
            .set_label(&gettext("Layer 3: first data detected during reading."));

        imp.read_status_dot.add_css_class("read-dot-done");
        imp.onion_core.add_css_class("onion-core-done");
    }

    #[allow(dead_code)]
    fn set_read_error(&self) {
        let imp = self.imp();

        self.clear_read_dot_classes();
        self.clear_onion_state_classes();

        imp.read_status_label
            .set_label(&gettext("Error reading file"));
        imp.tree_summary_label
            .set_label(&gettext("The project tree could not be generated."));
        imp.current_layer_label
            .set_label(&gettext("Current layer: error"));
        imp.onion_layer_detail_label
            .set_label(&gettext("The selected layer could not be read."));

        imp.read_status_dot.add_css_class("read-dot-error");
    }
}

fn tree_branch_parts(branch: &str) -> (i32, &'static str) {
    let connector_index = branch
        .chars()
        .position(|ch| ch == '├' || ch == '└')
        .unwrap_or(0);
    let depth = (connector_index / 4) as i32;
    let connector = if branch.contains('└') {
        "└─"
    } else if branch.contains('├') {
        "├─"
    } else {
        ""
    };

    (depth, connector)
}

fn demo_layers(
    count: usize,
    terminal_state: LayerState,
    terminal_label: String,
    current_label: String,
    detail: String,
) -> Vec<LayerSpec> {
    let mut layers = Vec::with_capacity(count);

    for index in 0..count {
        let layer_number = index + 1;
        let state = if index + 1 == count {
            terminal_state
        } else if index < 2 {
            LayerState::Done
        } else {
            LayerState::Idle
        };

        let label = if index + 1 == count {
            terminal_label.clone()
        } else {
            gettext(&format!("Layer {}", layer_number))
        };

        let current = if index + 1 == count {
            current_label.clone()
        } else {
            gettext(&format!("Current layer: {}", layer_number))
        };

        let detail_text = if index + 1 == count {
            detail.clone()
        } else {
            gettext(&format!("Layer {} in the analysis chain.", layer_number))
        };

        layers.push(LayerSpec {
            label,
            current_label: current,
            detail: detail_text,
            state,
        });
    }

    layers
}
