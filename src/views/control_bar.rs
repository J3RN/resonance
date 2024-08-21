/* control_bar.rs
 *
 * SPDX-FileCopyrightText: 2023 nate-xyz
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use adw::prelude::*;
use adw::subclass::prelude::*;

use gtk::{gdk, gio, glib, glib::clone, CompositeTemplate};

use log::debug;
use std::{cell::Cell, cell::RefCell, rc::Rc};

use crate::i18n::i18n;
use crate::model::track::Track;
use crate::player::queue::RepeatMode;
use crate::util::{model, player, seconds_to_string, settings_manager};
use crate::views::art::rounded_album_art::RoundedAlbumArt;

use super::scale::Scale;
use super::volume_widget::VolumeWidget;
use super::window::WindowPage;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/io/github/nate_xyz/Resonance/control_bar.ui")]
    pub struct ControlBarPriv {
        #[template_child(id = "volume_widget")]
        pub volume_widget: TemplateChild<VolumeWidget>,

        #[template_child(id = "prog_bar")]
        pub prog_bar: TemplateChild<gtk::ProgressBar>,

        #[template_child(id = "action_bar")]
        pub action_bar: TemplateChild<gtk::ActionBar>,

        #[template_child(id = "art_bin")]
        pub art_bin: TemplateChild<adw::Bin>,

        #[template_child(id = "track_info_button")]
        pub track_info_button: TemplateChild<gtk::Button>,

        #[template_child(id = "previous_button")]
        pub previous_button: TemplateChild<gtk::Button>,

        #[template_child(id = "play_button")]
        pub play_button: TemplateChild<gtk::Button>,

        #[template_child(id = "next_button")]
        pub next_button: TemplateChild<gtk::Button>,

        #[template_child(id = "spent_time_label")]
        pub spent_time_label: TemplateChild<gtk::Label>,

        #[template_child(id = "duration_label")]
        pub duration_label: TemplateChild<gtk::Label>,

        #[template_child(id = "play_pause_image")]
        pub play_pause_image: TemplateChild<gtk::Image>,

        #[template_child(id = "track_name_label")]
        pub track_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label")]
        pub album_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "artist_name_label")]
        pub artist_name_label: TemplateChild<gtk::Label>,

        #[template_child(id = "track_name_label_small")]
        pub track_name_label_small: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label_small")]
        pub album_name_label_small: TemplateChild<gtk::Label>,

        #[template_child(id = "artist_name_label_small")]
        pub artist_name_label_small: TemplateChild<gtk::Label>,

        #[template_child(id = "track_name_label_big")]
        pub track_name_label_big: TemplateChild<gtk::Label>,

        #[template_child(id = "album_name_label_big")]
        pub album_name_label_big: TemplateChild<gtk::Label>,

        #[template_child(id = "artist_name_label_big")]
        pub artist_name_label_big: TemplateChild<gtk::Label>,

        #[template_child(id = "repeat_button")]
        pub repeat_button: TemplateChild<gtk::Button>,

        #[template_child(id = "shuffle_button")]
        pub shuffle_button: TemplateChild<gtk::Button>,

        #[template_child(id = "loop_button")]
        pub loop_button: TemplateChild<gtk::Button>,

        #[template_child(id = "go_to_queue_button")]
        pub go_to_queue_button: TemplateChild<gtk::Button>,

        #[template_child(id = "progress_scale")]
        pub progress_scale: TemplateChild<Scale>,

        #[template_child(id = "leaflet")]
        pub leaflet: TemplateChild<adw::Leaflet>,

        #[template_child(id = "scale_box")]
        pub scale_box: TemplateChild<gtk::Box>,

        #[template_child(id = "popover")]
        pub popover: TemplateChild<gtk::PopoverMenu>,

        pub track: RefCell<Option<Rc<Track>>>,
        pub window_page: Cell<WindowPage>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ControlBarPriv {
        const NAME: &'static str = "ControlBar";
        type Type = super::ControlBar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ControlBarPriv {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().initialize();
        }
    }

    impl WidgetImpl for ControlBarPriv {}
    impl BinImpl for ControlBarPriv {}
    impl ControlBarPriv {}
}

glib::wrapper! {
    pub struct ControlBar(ObjectSubclass<imp::ControlBarPriv>)
    @extends gtk::Widget, adw::Bin;
}

impl ControlBar {
    pub fn new() -> ControlBar {
        let control_bar: ControlBar = glib::Object::builder::<ControlBar>().build();
        control_bar
    }

    pub fn initialize(&self) {
        let imp = self.imp();

        self.scale().set_id("CONTROL BAR SCALE.");
        self.scale().initialize();

        self.bind_state();
        self.button_connections();
        self.create_menu();

        let settings = settings_manager();
        let initial_repeat_mode_int = settings.int("repeat-mode");
        let repeat_mode = match initial_repeat_mode_int {
            0 => RepeatMode::Normal,
            1 => RepeatMode::Loop,
            2 => RepeatMode::LoopSong,
            3 => RepeatMode::Shuffle,
            _ => RepeatMode::Normal,
        };
        debug!("REPEAT MODE CONTROL BAR {:?}", repeat_mode);
        self.set_repeat_mode_ui(repeat_mode);

        imp.popover.set_parent(self);

        imp.leaflet.connect_notify_local(
            Some("folded"),
            clone!(@weak self as this => move |_leaflet, _folded| {
                let imp = this.imp();
                let folded = imp.leaflet.is_folded();
                imp.prog_bar.set_visible(folded);
            }),
        );

        let ctrl = gtk::GestureClick::new();
        ctrl.connect_released(
            clone!(@strong self as this => move |_gesture, _n_press, x, _y| {
                let imp = this.imp();
                if  x > 0.0 && x < this.width() as f64 {
                    let player = player();
                    let scale_ratio = x /  imp.prog_bar.width() as f64;
                    let time_position = player.state().duration() * scale_ratio;
                    player.set_track_position(time_position);
                }
            }),
        );
        imp.prog_bar.add_controller(ctrl);

        let ctrl = gtk::GestureClick::new();
        ctrl.connect_unpaired_release(
            clone!(@strong self as this => move |_gesture_click, x, y, button, _sequence| {
                let imp = this.imp();
                if button == gdk::BUTTON_SECONDARY {
                    let height = this.height();
                    let width = this.width() / 2;
                    let y = y as i32;
                    let x = x as i32;
                    imp.popover.set_offset(x - width, y - height);
                    imp.popover.popup();
                }
            }),
        );
        self.add_controller(ctrl);
    }

    // Bind the PlayerState to the UI
    fn bind_state(&self) {
        debug!("bind_state");
        let player = player();

        // Update current track
        player.state().connect_notify_local(
            Some("song"),
            clone!(@strong self as this => move |_, _| {
                this.update_current_track();
            }),
        );

        player.state().connect_notify_local(
            Some("playing"),
            clone!(@strong self as this => move |_, _| {
                this.sync_playing();
            }),
        );

        player.state().connect_notify_local(
            Some("position"),
            clone!(@weak self as this => move |_, _| {
                this.update_position();
            }),
        );

        let scale = self.scale();
        scale.connect_notify_local(
            Some("time-position"),
            clone!(@weak self as this => move |_, _| {
                this.update_time_position();
            }),
        );

        player.state().connect_local(
            "queue-repeat-mode",
            false,
            clone!(@strong self as this => move |value| {
                let mode = value.get(1).unwrap().get::<RepeatMode>().ok().unwrap();
                this.set_repeat_mode_ui(mode);
                None
            }),
        );
    }

    fn set_repeat_mode_ui(&self, mode: RepeatMode) {
        let imp = self.imp();
        match mode {
            RepeatMode::Normal => {
                imp.shuffle_button.remove_css_class("suggested-action");
                imp.loop_button.remove_css_class("suggested-action");
                imp.repeat_button.remove_css_class("suggested-action");
            }
            RepeatMode::Loop => {
                imp.shuffle_button.remove_css_class("suggested-action");
                imp.loop_button.add_css_class("suggested-action");
                imp.repeat_button.remove_css_class("suggested-action");
            }
            RepeatMode::LoopSong => {
                imp.shuffle_button.remove_css_class("suggested-action");
                imp.loop_button.remove_css_class("suggested-action");
                imp.repeat_button.add_css_class("suggested-action");
            }
            RepeatMode::Shuffle => {
                imp.shuffle_button.add_css_class("suggested-action");
                imp.loop_button.remove_css_class("suggested-action");
                imp.repeat_button.remove_css_class("suggested-action");
            }
        }
    }

    fn sync_playing(&self) {
        let imp = self.imp();
        if player().state().playing() {
            debug!("playing");
            imp.play_pause_image
                .set_icon_name(Some("media-playback-pause-symbolic"));
            imp.play_button.set_tooltip_text(Some(&i18n("Pause")));
        } else {
            debug!("paused");
            imp.play_pause_image
                .set_icon_name(Some("media-playback-start-symbolic"));
            imp.play_button.set_tooltip_text(Some(&i18n("Play")));
        }
    }

    fn button_connections(&self) {
        let imp = self.imp();
        imp.play_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().toggle_play_pause();
            }),
        );

        imp.previous_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().prev();
            }),
        );

        imp.next_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().next();
            }),
        );

        imp.loop_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().on_repeat_change(RepeatMode::Loop);
            }),
        );

        imp.repeat_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().on_repeat_change(RepeatMode::LoopSong);
            }),
        );

        imp.shuffle_button.connect_clicked(
            clone!(@strong self as this => @default-panic, move |_button| {
                player().queue().on_repeat_change(RepeatMode::Shuffle);
            }),
        );
    }

    fn update_current_track(&self) {
        self.imp().track.replace(player().state().current_track());
        self.update_view();
    }

    pub fn update_view(&self) {
        let imp = self.imp();
        imp.spent_time_label.set_label("0:00");

        if let Some(track) = imp.track.borrow().as_ref() {
            imp.track_name_label.set_label(&track.title());
            imp.album_name_label.set_label(&track.album());
            imp.artist_name_label.set_label(&track.artist());
            imp.track_name_label_small.set_label(&track.title());
            imp.album_name_label_small.set_label(&track.album());
            imp.artist_name_label_small.set_label(&track.artist());
            imp.track_name_label_big.set_label(&track.title());
            imp.album_name_label_big.set_label(&track.album());
            imp.artist_name_label_big.set_label(&track.artist());
            imp.duration_label
                .set_label(&format!(" / {}", seconds_to_string(track.duration())));

            self.sync_prev_next();
            self.load_art(track.cover_art_option());
            self.set_revealed(true);
        } else {
            imp.duration_label.set_label(" / 0:00");
            //imp.art_bin.set_child(gtk::Widget::NONE);
            imp.art_bin.hide();
            imp.previous_button.set_sensitive(false);
            imp.next_button.set_sensitive(false);
            self.set_revealed(false);
        }
    }

    fn load_art(&self, art: Option<i64>) {
        let imp = self.imp();

        if let Some(id) = art {
            if let Ok(art) = self.load_image(id) {
                imp.art_bin.set_child(Some(&art));
                imp.art_bin.show();
                return;
            }
        }

        //imp.art_bin.set_child(gtk::Widget::NONE);
        imp.art_bin.hide();
    }

    fn load_image(&self, cover_art_id: i64) -> Result<RoundedAlbumArt, String> {
        let cover_art = model().cover_art(cover_art_id)?;
        let pixbuf = cover_art.pixbuf()?;

        let art = RoundedAlbumArt::new(90);
        art.load(pixbuf);
        Ok(art)
    }

    fn update_position(&self) {
        let imp = self.imp();

        let player = player();
        let position = player.state().position() as f64;
        let duration = player.state().duration() as f64;
        let ratio = position / duration;
        imp.prog_bar.set_fraction(ratio);
    }

    fn update_time_position(&self) {
        let imp = self.imp();
        let position = self.scale().time_position() as f64;
        imp.spent_time_label.set_label(&seconds_to_string(position));
    }

    fn sync_prev_next(&self) {
        let imp = self.imp();
        let player = player();
        let has_next = player.has_next();
        let has_previous = player.has_previous();
        imp.play_button.set_sensitive(has_next);
        imp.previous_button.set_sensitive(has_previous);
        imp.next_button.set_sensitive(has_next);
    }

    fn empty(&self) -> bool {
        match self.imp().track.borrow().as_ref() {
            Some(_) => false,
            None => true,
        }
    }

    fn revealed(&self) -> bool {
        self.imp().action_bar.is_revealed()
    }

    pub fn set_revealed(&self, reveal: bool) {
        let imp = self.imp();
        let playing = player().state().playing();

        if self.empty() || self.window_page() == WindowPage::Queue {
            debug!("CONTROL BAR HIDE");
            if self.revealed() {
                imp.action_bar.set_revealed(false);
                imp.prog_bar.set_visible(false);
            }
            return;
        }

        if (reveal == playing || reveal == !self.empty()) && reveal != self.revealed() {
            imp.action_bar.set_revealed(reveal);

            let folded = imp.leaflet.is_folded();
            if reveal && folded {
                imp.prog_bar.set_visible(folded);
            }
        }
    }

    pub fn set_window_page(&self, state: WindowPage) {
        self.imp().window_page.set(state);
    }

    fn window_page(&self) -> WindowPage {
        self.imp().window_page.get()
    }

    pub fn track(&self) -> Rc<Track> {
        self.imp().track.borrow().as_ref().unwrap().clone()
    }

    fn scale(&self) -> &Scale {
        &self.imp().progress_scale
    }

    pub fn go_to_queue_button(&self) -> &gtk::Button {
        &self.imp().go_to_queue_button
    }

    pub fn track_info_button(&self) -> &gtk::Button {
        &self.imp().track_info_button
    }

    fn create_menu(&self) {
        let imp = self.imp();

        let main = gio::Menu::new();
        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&i18n("Toggle Play Pause")), None);
        menu_item.set_action_and_target_value(Some("win.toggle-play-pause"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&i18n("Previous")), None);
        menu_item.set_action_and_target_value(Some("win.prev"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&i18n("Next")), None);
        menu_item.set_action_and_target_value(Some("win.next"), None);
        menu.append_item(&menu_item);

        let menu_item = gio::MenuItem::new(Some(&i18n("End Queue")), None);
        menu_item.set_action_and_target_value(Some("win.end-queue"), None);
        menu.append_item(&menu_item);

        main.append_section(Some(&i18n("Playback")), &menu);

        let menu = gio::Menu::new();

        let menu_item = gio::MenuItem::new(Some(&i18n("Go to Queue")), None);
        menu_item.set_action_and_target_value(Some("win.go-to-queue"), None);
        menu.append_item(&menu_item);

        main.append_section(Some(&i18n("Navigate")), &menu);

        imp.popover.set_menu_model(Some(&main));
    }
}
