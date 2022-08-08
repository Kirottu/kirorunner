use std::{cell::RefCell, collections::HashMap, io::stdin, rc::Rc};

use gdk::keys::constants;
use gio::prelude::*;
use gtk::prelude::*;

struct Entry {
    pub rating: f64,
    pub hidden: bool,
    pub name: String,
}

const INPUT_BOX_NAME: &str = "input-box";
const LIST_BOX_NAME: &str = "list-box";
const SCROLL_WINDOW_NAME: &str = "scroll-window";
const WINDOW_NAME: &str = "window";
const ENTRY_LABEL_NAME: &str = "entry-label";
const MAIN_BOX_NAME: &str = "main-box";

fn main() {
    // Create the application
    let app = gtk::Application::new(Some("com.kirottu.kirorunner"), Default::default());

    app.connect_activate(|app| activate(app));

    app.run();
}

/// Main activation function
fn activate(app: &gtk::Application) {
    // Load our custom CSS, if it exists
    match std::env::var("KIRORUNNER_CSS") {
        Ok(var) => {
            let provider = gtk::CssProvider::new();
            match provider.load_from_path(&var) {
                Ok(_) => (),
                Err(why) => {
                    eprintln!("Failed to load CSS: {}", why);
                }
            }
            gtk::StyleContext::add_provider_for_screen(
                &gdk::Screen::default().expect("Failed to init GTK CSS provider"),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        Err(_) => (),
    }

    // Create the main window
    let window = gtk::ApplicationWindow::builder()
        .name(WINDOW_NAME)
        .application(app)
        .build();

    // Set the window size
    window.set_size_request(800, 400);

    // Layer shell stuff
    gtk_layer_shell::init_for_window(&window);
    gtk_layer_shell::set_keyboard_mode(&window, gtk_layer_shell::KeyboardMode::Exclusive);
    gtk_layer_shell::set_layer(&window, gtk_layer_shell::Layer::Overlay);
    gtk_layer_shell::set_anchor(&window, gtk_layer_shell::Edge::Top, true);

    // The main VBox of the program
    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(5)
        .name(MAIN_BOX_NAME)
        .build();
    // The box where you enter your search
    let entry_box = gtk::Entry::builder()
        .has_focus(true)
        .margin_end(5)
        .margin_start(5)
        .name(INPUT_BOX_NAME)
        .build();
    let scroll_window = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .vscrollbar_policy(gtk::PolicyType::External)
        .name(SCROLL_WINDOW_NAME)
        .build();
    let list_box = Rc::new(
        gtk::ListBox::builder()
            .hexpand(true)
            .vexpand(true)
            .name(LIST_BOX_NAME)
            .build(),
    );

    let entries = Rc::new(RefCell::new(
        stdin()
            .lines()
            .map(|line| {
                let row = gtk::ListBoxRow::builder().build();
                let label = gtk::Label::builder()
                    .label(line.as_ref().unwrap())
                    .height_request(30)
                    .hexpand(true)
                    .halign(gtk::Align::Start)
                    .name(ENTRY_LABEL_NAME)
                    .build();
                row.add(&label);
                list_box.add(&row);
                (
                    row,
                    Entry {
                        rating: 1.0,
                        hidden: false,
                        name: line.unwrap(),
                    },
                )
            })
            .collect::<HashMap<gtk::ListBoxRow, Entry>>(),
    ));

    window.connect_key_press_event(
        closure::closure!(clone list_box, clone entries, |window, event| match event.keyval() {
            constants::Escape => {
                window.close();
                Inhibit(true)
            }
            constants::Return => {
                match list_box.selected_row() {
                    Some(row) => {
                        let entries = entries.borrow();
                        println!("{}", entries[&row].name);
                        window.close();
                        Inhibit(true)
                    }
                    None => {
                        Inhibit(false)
                    }
                }
            }
            constants::Down => {
                if list_box.row_at_index(0) == list_box.selected_row() {
                    match list_box.row_at_index(1) {
                        Some(row) => list_box.select_row(Some(&row)),
                        None => (),
                    }
                }
                Inhibit(false)
            }
            _ => Inhibit(false),
        }),
    );


    entry_box.connect_changed(closure::closure!(clone entries, clone list_box, |entry_box| {
        {
            // Get a mutable reference to the entries
            let mut entries = entries.borrow_mut();
            // Store the highest rated entry for highlighting later
            let mut highest_rating: Option<(&gtk::ListBoxRow, f64)> = None;
            for (row, entry) in entries.iter_mut() {
                if entry_box.text().len() == 0 {
                    entry.rating = 1.0;
                    entry.hidden = false;
                    continue;
                }
                // String similarity magic
                entry.rating = strsim::jaro_winkler(&entry_box.text().to_lowercase(), &entry.name.to_lowercase());
                // Update the highest rated item
                match highest_rating {
                    Some(_highest_rating) => {
                        if entry.rating > _highest_rating.1 {
                            highest_rating = Some((row, entry.rating));
                        }
                    }
                    None => {
                        highest_rating = Some((row, entry.rating));
                    }
                }
                // Arbitrary low-bar for results
                if entry.rating < 0.1 {
                    entry.hidden = true;
                } else {
                    entry.hidden = false;
                }
            }
            // Set the selected item, if one exists
            match highest_rating {
                Some(highest_rating) => {
                    list_box.select_row(Some(highest_rating.0));
                }
                None => (),
            }
        }

        // Update the sort and filter of the ListBox
        list_box.invalidate_filter();
        list_box.invalidate_sort();
    }));

    // Sort them according to the rating
    list_box.set_sort_func(Some(Box::new(closure::closure!(clone entries, |a, b| {
        let entries = entries.borrow();
        match entries[b].rating.partial_cmp(&entries[a].rating) {
            Some(ordering) => ordering as i32,
            None => 0,
        }
    }))));
    // Filter them according to them being hidden or not
    list_box.set_filter_func(Some(Box::new(closure::closure!(clone entries, |a| {
        let entries = entries.borrow();
        !entries[a].hidden
    }))));

    vbox.add(&entry_box);
    scroll_window.add(&*list_box);
    vbox.add(&scroll_window);

    window.add(&vbox);
    window.show_all();
}
