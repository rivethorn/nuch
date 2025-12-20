use std::path::PathBuf;

use cursive::{
    View,
    view::Resizable,
    views::{Dialog, DummyView, LinearLayout, SelectView},
};
use cursive_hjkl::HjklToDirectionWrapperView;

fn hjkl<V: View>(view: V) -> HjklToDirectionWrapperView<V> {
    HjklToDirectionWrapperView::new(view)
}

pub fn run_tui(blogs: Vec<PathBuf>, content: Vec<PathBuf>) {
    let mut siv = cursive::default();

    let mut published = SelectView::new();
    published.add_all_str(
        content
            .iter()
            .map(|path| path.file_name())
            .map(|file| file.unwrap().to_str().unwrap())
            .collect::<Vec<&str>>(),
    );

    let mut publishable = SelectView::new();
    publishable.add_all_str(
        blogs
            .iter()
            .map(|path| path.file_name())
            .map(|file| file.unwrap().to_str().unwrap())
            .collect::<Vec<&str>>(),
    );

    siv.add_layer(hjkl(
        Dialog::around(
            LinearLayout::horizontal()
                .child(Dialog::around(published).title("Published Posts"))
                .child(DummyView.fixed_width(4))
                .child(Dialog::around(publishable).title("Publishable Posts"))
                .min_height(10),
        )
        .title("NUxt Content Handler")
        .button("Quit", |s| s.quit()), // .full_screen(),
    ));

    siv.add_global_callback('q', |s| s.quit());

    siv.run();
}
