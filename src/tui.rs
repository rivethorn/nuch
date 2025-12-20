use std::path::PathBuf;

use cursive::{
    Rect, Vec2, View,
    view::{Resizable, Scrollable},
    views::{
        Dialog, DummyView, FixedLayout, Layer, LinearLayout, OnLayoutView, SelectView, TextView,
    },
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
                .child(
                    Dialog::around(published)
                        .title("Published Posts")
                        .scrollable(),
                )
                .child(DummyView.fixed_width(4))
                .child(
                    Dialog::around(publishable)
                        .title("Publishable Posts")
                        .scrollable(),
                )
                .min_height(10),
        )
        .title("NUxt Content Handler")
        .button("Quit", |s| s.quit()),
    ));

    siv.add_global_callback('q', |s| {
        s.add_layer(
            Dialog::text("Do you want to quit?")
                .button("Yes", |s| s.quit())
                .button("No", |s| {
                    s.pop_layer();
                }),
        );
    });

    let status = LinearLayout::horizontal().child(TextView::new("vim"));

    siv.screen_mut().add_transparent_layer(
        OnLayoutView::new(
            FixedLayout::new().child(
                Rect::from_point(Vec2::zero()),
                Layer::new(status).full_width(),
            ),
            |layout, size| {
                layout.set_child_position(0, Rect::from_size((0, size.y - 1), (size.x, 1)));
                layout.layout(size);
            },
        )
        .full_screen(),
    );

    siv.run();
}
