use crate::codewars_requests::get_completed;
use crate::db::{ChatMessage, CodeUser, UserId};
use crate::error::MainError;
use crate::message_parse::kata_name;
use plotlib::{page, repr, style, view};
use resvg::usvg;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use svg;
use uuid;

pub async fn compute_stats(
    users: HashMap<UserId, CodeUser>,
    messages: Vec<ChatMessage>,
) -> Result<PathBuf, MainError> {
    let mut user_stats = Vec::new();

    for user in users.values() {
        let solved_in_scala: Vec<_> = get_completed(user.codewars_name.as_str())
            .await?
            .into_iter()
            .filter(|k| k.completed_languages.contains(&"scala".to_owned()))
            .collect();
        let sent_to_chat = messages
            .iter()
            .filter(|msg| msg.from == user.telegram_id)
            .count();
        user_stats.push((user.clone(), solved_in_scala.len(), sent_to_chat))
    }
    let plot = repr::BarChart::new(10.);
    let view = view::CategoricalView::new()
        .add(plot)
        .x_ticks(&["shit".to_owned(), "pinus".to_owned()]);
    Ok(to_image(page::Page::single(&view)))
}

fn to_image(page: page::Page) -> PathBuf {
    let bytes = Vec::new();
    svg::write(bytes, page.to_svg().unwrap());
    let svg = usvg::Tree::from_data(bytes.as_slice(), &usvg::Options::default()).unwrap();
    let mut img = resvg::default_backend()
        .render_to_image(&svg, &resvg::Options::default())
        .unwrap();
    let path = Path::new(format!("tmp/img_{}.png", uuid::Uuid::new_v4()).as_str());
    img.save_png(path);
    PathBuf::from(path)
}
