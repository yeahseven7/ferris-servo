//! ferris-servo — a real-engine browser shell.
//!
//! GTK4 chrome (tab strip + address bar + nav buttons) wrapped around the
//! **Servo** web engine via the `servo-gtk` widget. Unlike the `ferris-browser`
//! toy engine, Servo is a production-aimed rendering engine (also written in
//! Rust), so real sites — JS, CSS, media — actually work here.
//!
//! The chrome is ours; the engine is Servo's. This is the "all-Rust" lane.

use std::cell::RefCell;
use std::ptr;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Box as GtkBox, Button, Entry, Label, Notebook,
          Orientation, Spinner, glib};
use servo_gtk::{LoadEvent, WebView};

const APP_ID: &str = "dev.yeahseven7.ferris-shell";
const HOME: &str = "https://example.com";

type Tabs = Rc<RefCell<Vec<WebView>>>;
type TabLabels = Rc<RefCell<Vec<Label>>>;

/// Add a scheme when missing so bare words like `example.com` navigate.
fn normalize(input: &str) -> String {
    let t = input.trim();
    if t.is_empty() {
        return "about:blank".to_string();
    }
    if t.contains("://") {
        t.to_string()
    } else {
        format!("https://{t}")
    }
}

fn active_webview(nb: &Notebook, tabs: &Tabs) -> Option<WebView> {
    nb.current_page()
        .and_then(|i| tabs.borrow().get(i as usize).cloned())
}

fn index_of(tabs: &Tabs, w: &WebView) -> Option<usize> {
    tabs.borrow().iter().position(|x| x == w)
}

/// Create a tab hosting a fresh [`WebView`] loaded with `url`, wire its
/// signals, and switch to it.
fn add_tab(
    nb: &Notebook,
    entry: &Entry,
    spinner: &Spinner,
    tabs: &Tabs,
    labels: &TabLabels,
    url: &str,
) {
    let web = WebView::new();
    web.set_hexpand(true);
    web.set_vexpand(true);

    // Tab label: title + a close button.
    let title = Label::new(Some("new tab"));
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.set_max_width_chars(18);
    let close = Button::from_icon_name("window-close-symbolic");
    close.set_tooltip_text(Some("close tab"));
    let label_box = GtkBox::new(Orientation::Horizontal, 4);
    label_box.append(&title);
    label_box.append(&close);

    let idx = nb.append_page(&web, Some(&label_box));
    nb.set_tab_reorderable(&web, true);
    tabs.borrow_mut().push(web.clone());
    labels.borrow_mut().push(title.clone());

    // --- close button: remove this tab by identity ---
    let (nb_c, tabs_c, labels_c) = (nb.clone(), tabs.clone(), labels.clone());
    let web_c = web.clone();
    close.connect_clicked(move |_| {
        if let Some(i) = index_of(&tabs_c, &web_c) {
            tabs_c.borrow_mut().remove(i);
            labels_c.borrow_mut().remove(i);
            nb_c.remove_page(Some(i as u32));
        }
    });

    // --- keep the address bar in sync with the active tab's real URI ---
    let (nb_u, tabs_u, entry_u) = (nb.clone(), tabs.clone(), entry.clone());
    let web_u = web.clone();
    web.connect_uri_notify(move |w| {
        let is_active = active_webview(&nb_u, &tabs_u)
            .map(|a| a == web_u)
            .unwrap_or(false);
        if is_active && let Some(uri) = w.uri() {
            entry_u.set_text(&uri);
        }
    });

    // --- reflect page title in the tab label ---
    let labels_t = labels.clone();
    let tabs_t = tabs.clone();
    let web_t = web.clone();
    web.connect_title_notify(move |w| {
        if let Some(i) = index_of(&tabs_t, &web_t) {
            let text = w
                .title()
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| glib::GString::from("new tab"));
            if let Some(l) = labels_t.borrow().get(i) {
                l.set_text(&text);
            }
        }
    });

    // --- spinner while the active tab loads ---
    let (nb_l, tabs_l, sp_l) = (nb.clone(), tabs.clone(), spinner.clone());
    let web_l = web.clone();
    web.connect_load_changed(move |w, event| {
        let is_active = active_webview(&nb_l, &tabs_l)
            .map(|a| a == web_l)
            .unwrap_or(false);
        if !is_active {
            return;
        }
        match event {
            LoadEvent::Started => sp_l.start(),
            LoadEvent::Finished => sp_l.stop(),
            _ => {}
        }
    });

    // --- popups / target=_blank open as new tabs ---
    let (nb_p, entry_p, sp_p, tabs_p, labels_p) =
        (nb.clone(), entry.clone(), spinner.clone(), tabs.clone(), labels.clone());
    web.connect_create_web_view(move |_, url| {
        add_tab(&nb_p, &entry_p, &sp_p, &tabs_p, &labels_p, url);
    });

    nb.set_current_page(Some(idx));
    web.load_url(&normalize(url));
}

fn build_window(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("ferris-servo")
        .default_width(1180)
        .default_height(820)
        .build();

    let vbox = GtkBox::new(Orientation::Vertical, 5);
    let toolbar = GtkBox::new(Orientation::Horizontal, 5);

    let back = Button::from_icon_name("go-previous-symbolic");
    let forward = Button::from_icon_name("go-next-symbolic");
    let reload = Button::from_icon_name("view-refresh-symbolic");
    let newtab = Button::from_icon_name("tab-new-symbolic");
    let spinner = Spinner::new();
    let entry = Entry::builder()
        .placeholder_text("enter a url…")
        .hexpand(true)
        .build();

    for b in [&back, &forward, &reload, &newtab] {
        toolbar.append(b);
    }
    toolbar.append(&entry);
    toolbar.append(&spinner);

    let nb = Notebook::new();
    nb.set_scrollable(true);
    nb.set_vexpand(true);

    let tabs: Tabs = Rc::new(RefCell::new(Vec::new()));
    let labels: TabLabels = Rc::new(RefCell::new(Vec::new()));

    // toolbar wiring acts on the active tab
    let (nb_b, tabs_b) = (nb.clone(), tabs.clone());
    back.connect_clicked(move |_| {
        if let Some(w) = active_webview(&nb_b, &tabs_b) {
            w.go_back();
        }
    });
    let (nb_f, tabs_f) = (nb.clone(), tabs.clone());
    forward.connect_clicked(move |_| {
        if let Some(w) = active_webview(&nb_f, &tabs_f) {
            w.go_forward();
        }
    });
    let (nb_r, tabs_r) = (nb.clone(), tabs.clone());
    reload.connect_clicked(move |_| {
        if let Some(w) = active_webview(&nb_r, &tabs_r) {
            w.reload();
        }
    });
    let (nb_n, entry_n, sp_n, tabs_n, labels_n) =
        (nb.clone(), entry.clone(), spinner.clone(), tabs.clone(), labels.clone());
    newtab.connect_clicked(move |_| {
        add_tab(&nb_n, &entry_n, &sp_n, &tabs_n, &labels_n, HOME);
    });

    // Enter in the address bar loads into the active tab
    let (nb_e, tabs_e) = (nb.clone(), tabs.clone());
    entry.connect_activate(move |e| {
        if let Some(w) = active_webview(&nb_e, &tabs_e) {
            w.load_url(&normalize(&e.text()));
        }
    });

    // switching tabs refreshes the address bar from that tab
    let (tabs_s, entry_s) = (tabs.clone(), entry.clone());
    nb.connect_switch_page(move |nb, _, page_num| {
        if let Some(w) = tabs_s.borrow().get(page_num as usize) {
            if let Some(uri) = w.uri() {
                entry_s.set_text(&uri);
            }
        }
        let _ = nb;
    });

    vbox.append(&toolbar);
    vbox.append(&nb);
    window.set_child(Some(&vbox));

    add_tab(&nb, &entry, &spinner, &tabs, &labels, HOME);
    window.present();
}

fn main() -> glib::ExitCode {
    // Hand off to the Servo runner subprocess if we were re-exec'd as one.
    servo_gtk::run_as_runner_if_requested();

    // GL loader setup (mirrors the upstream servo-gtk example).
    let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }.unwrap();
    epoxy::load_with(|name| {
        unsafe { library.get::<_>(name.as_bytes()) }
            .map(|symbol| *symbol)
            .unwrap_or(ptr::null())
    });

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| build_window(app));
    app.run()
}
