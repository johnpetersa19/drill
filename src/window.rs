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

const MIN_VISIBLE_LAYERS: usize = 2;
const MAX_VISIBLE_LAYERS: usize = 10;
const DEFAULT_ONION_SIZE: i32 = 340;
const MIN_ONION_SIZE: i32 = 260;
const MAX_ONION_SIZE: i32 = 560;
const ONION_CORE_SIZE: i32 = 84;
const ONION_LAYER_BADGE_WIDTH: i32 = 18;
const ONION_LAYER_BADGE_HEIGHT: i32 = 16;
const ONION_LAYER_BADGE_GAP: usize = 2;
const ONION_CORE_RING_GAP: i32 = 34;
/// Inset from the ring's right edge so the badge sits fully inside the ring.
const ONION_LAYER_BADGE_INSET: f64 = 4.0;

#[derive(Clone)]
struct TreeItem {
    branch: &'static str,
    icon: &'static str,
    name: Cow<'static, str>,
}

#[derive(Debug, Clone)]
pub(super) struct LayerSpec {
    number: usize,
    label: String,
    current_label: String,
    detail: String,
    state: LayerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LayerState {
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
        pub read_status_dot: TemplateChild<adw::Bin>,

        #[template_child]
        pub read_status_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub tree_rows_box: TemplateChild<gtk::Box>,

        #[template_child]
        pub tree_summary_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub main_paned: TemplateChild<gtk::Paned>,

        #[template_child]
        pub current_layer_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub onion_layer_detail_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub onion_layers_fixed: TemplateChild<gtk::Fixed>,

        #[template_child]
        pub onion_overlay: TemplateChild<gtk::Overlay>,

        #[template_child]
        pub onion_core: TemplateChild<adw::Bin>,

        #[template_child]
        pub zoom_in_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub zoom_out_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub zoom_page_label: TemplateChild<gtk::Label>,

        /// Full list of layers from the last call to `set_onion_layers_full`.
        pub(super) all_layers: RefCell<Vec<LayerSpec>>,

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
            obj.restore_window_state();
            obj.setup_window_actions();
            obj.setup_window_state_saving();
            obj.setup_dynamic_layers();
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

    fn settings() -> gio::Settings {
        gio::Settings::new("io.github.johnpetersa.Drill")
    }

    fn restore_window_state(&self) {
        let settings = Self::settings();
        self.set_default_size(settings.int("window-width"), settings.int("window-height"));

        if settings.boolean("window-maximized") {
            self.maximize();
        }
    }

    fn setup_window_state_saving(&self) {
        self.connect_close_request(|window| {
            window.save_window_state();
            glib::Propagation::Proceed
        });
    }

    fn save_window_state(&self) {
        let settings = Self::settings();
        let (width, height) = self.default_size();

        let _ = settings.set_int("window-width", width);
        let _ = settings.set_int("window-height", height);
        let _ = settings.set_boolean("window-maximized", self.is_maximized());
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

    fn setup_dynamic_layers(&self) {
        let imp = self.imp();

        imp.main_paned.connect_position_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.refresh_onion_viewport();
            }
        ));

        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                window.refresh_onion_viewport();
            }
        ));
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
                let visible_layers = window.visible_layer_count(total);
                let new_offset = (offset + 1).min(total.saturating_sub(visible_layers));
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

    fn onion_size(&self) -> i32 {
        let imp = self.imp();
        let paned_width = imp.main_paned.width();
        let right_width = paned_width.saturating_sub(imp.main_paned.position());
        let available_width = if right_width > 0 {
            right_width.saturating_sub(64)
        } else {
            DEFAULT_ONION_SIZE
        };

        available_width.clamp(MIN_ONION_SIZE, MAX_ONION_SIZE)
    }

    fn visible_layer_count(&self, total: usize) -> usize {
        if total == 0 {
            return 0;
        }

        let min_ring_size = ONION_CORE_SIZE + ONION_CORE_RING_GAP * 2;
        let available_ring_space =
            self.onion_size().saturating_sub(44 + min_ring_size) as usize / 2;
        let by_size =
            (available_ring_space / (ONION_LAYER_BADGE_WIDTH as usize + ONION_LAYER_BADGE_GAP)) + 1;

        by_size
            .clamp(MIN_VISIBLE_LAYERS, MAX_VISIBLE_LAYERS)
            .min(total)
    }

    /// Re-render rings using the current offset and update zoom button sensitivity.
    fn refresh_onion_viewport(&self) {
        let imp = self.imp();
        let all = imp.all_layers.borrow();
        let total = all.len();
        let visible_layers = self.visible_layer_count(total);
        let max_offset = total.saturating_sub(visible_layers);
        let offset = imp.layer_offset.get().min(max_offset);
        imp.layer_offset.set(offset);

        let end = (offset + visible_layers).min(total);
        let visible: Vec<LayerSpec> = all[offset..end].to_vec();

        drop(all); // release borrow before calling set_onion_rings
        self.set_onion_rings(&visible);

        let imp = self.imp();
        let total = imp.all_layers.borrow().len();
        let offset = imp.layer_offset.get();
        let visible_layers = self.visible_layer_count(total);

        imp.zoom_out_button.set_sensitive(offset > 0);
        imp.zoom_in_button
            .set_sensitive(offset + visible_layers < total);

        if total == 0 {
            imp.zoom_page_label.set_label("–");
        } else {
            let first = offset + 1;
            let last = (offset + visible_layers).min(total);
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
        let onion_size = self.onion_size();
        let outer_size = (onion_size - 44) as f64;
        let inner_size = (ONION_CORE_SIZE + ONION_CORE_RING_GAP * 2) as f64;
        imp.onion_overlay.set_width_request(onion_size);
        imp.onion_overlay.set_height_request(onion_size);
        imp.onion_layers_fixed.set_width_request(onion_size);
        imp.onion_layers_fixed.set_height_request(onion_size);

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

            let is_edge_layer = index == 0 || index + 1 == layers.len();
            match layer.state {
                LayerState::Active => ring.add_css_class("onion-layer-active"),
                LayerState::Done if is_edge_layer => ring.add_css_class("onion-layer-done"),
                LayerState::Done | LayerState::Idle => {}
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

            let offset = ((onion_size - size) / 2) as f64;
            imp.onion_layers_fixed.put(&ring, offset, offset);

            let number = gtk::Label::new(Some(&layer.number.to_string()));
            number.set_halign(gtk::Align::Center);
            number.set_valign(gtk::Align::Center);
            number.set_width_request(ONION_LAYER_BADGE_WIDTH);
            number.set_height_request(ONION_LAYER_BADGE_HEIGHT);
            number.set_tooltip_text(Some(layer.label.as_str()));
            number.add_css_class("onion-layer-number");

            match layer.state {
                LayerState::Active => number.add_css_class("onion-layer-number-active"),
                LayerState::Done if is_edge_layer => {
                    number.add_css_class("onion-layer-number-done")
                }
                LayerState::Done | LayerState::Idle => {}
            }

            let click = gtk::GestureClick::new();
            let window = self.downgrade();
            let current_label = layer.current_label.clone();
            let detail = layer.detail.clone();
            click.connect_pressed(move |_, _, _, _| {
                if let Some(window) = window.upgrade() {
                    window.show_onion_layer(&current_label, &detail);
                }
            });
            number.add_controller(click);

            let badge_width = ONION_LAYER_BADGE_WIDTH as f64;
            let badge_height = ONION_LAYER_BADGE_HEIGHT as f64;
            // Place the badge fully inside the ring: right edge of badge sits
            // ONION_LAYER_BADGE_INSET pixels from the right border of the ring.
            let ring_right = offset + size as f64;
            let number_x = ring_right - badge_width - ONION_LAYER_BADGE_INSET;
            let number_y = onion_size as f64 / 2.0 - badge_height / 2.0;
            imp.onion_layers_fixed.put(&number, number_x, number_y);
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
        let layer_count = PROJECT_TREE_ITEMS.len().max(MAX_VISIBLE_LAYERS);
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

/// Returns a translated string like "Layer {n}" using the template
/// `"Layer %d"` extracted via gettext.
///
/// The literal `"Layer %d"` is the canonical msgid registered in the .pot.
/// At runtime we translate it then replace `%d` with the number.
fn layer_label(n: usize) -> String {
    // TRANSLATORS: %d is replaced with the layer number (e.g. "Layer 4")
    gettext("Layer %d").replace("%d", &n.to_string())
}

/// Returns a translated string like "Current layer: {n}".
///
/// msgid: `"Current layer: %d"`
fn current_layer_label(n: usize) -> String {
    // TRANSLATORS: %d is replaced with the layer number (e.g. "Current layer: 4")
    gettext("Current layer: %d").replace("%d", &n.to_string())
}

/// Returns a translated string like "Layer {n} in the analysis chain."
///
/// msgid: `"Layer %d in the analysis chain."`
fn layer_detail(n: usize) -> String {
    // TRANSLATORS: %d is replaced with the layer number (e.g. "Layer 4 in the analysis chain.")
    gettext("Layer %d in the analysis chain.").replace("%d", &n.to_string())
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
            layer_label(layer_number)
        };

        let current = if index + 1 == count {
            current_label.clone()
        } else {
            current_layer_label(layer_number)
        };

        let detail_text = if index + 1 == count {
            detail.clone()
        } else {
            layer_detail(layer_number)
        };

        layers.push(LayerSpec {
            number: layer_number,
            label,
            current_label: current,
            detail: detail_text,
            state,
        });
    }

    layers
}
