use std::{env::home_dir, error::Error, fs::read_dir, path::PathBuf};

use cursive::{
    View,
    view::Resizable,
    views::{Dialog, DummyView, LinearLayout, SelectView},
};
use cursive_hjkl::HjklToDirectionWrapperView;

fn hjkl<V: View>(view: V) -> HjklToDirectionWrapperView<V> {
    HjklToDirectionWrapperView::new(view)
}

fn handle_paths(home_dir: PathBuf) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Box<dyn Error>> {
    const BLOG_DIR: &str = "Documents/blog";
    const CONTENT_DIR: &str = "Documents/GitHub/hq/content/writings";
    let blog_dir = home_dir.join(BLOG_DIR);
    let content_dir = home_dir.join(CONTENT_DIR);

    let blogs = read_dir(blog_dir)?
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == "md") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let content = read_dir(content_dir)?
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            if path.extension().map_or(false, |ext| ext == "md") {
                Some(path)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok((blogs, content))
}

fn main() {
    let home_dir = home_dir().unwrap();
    let (blogs, content) = handle_paths(home_dir).unwrap();
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

    siv.run();
}
