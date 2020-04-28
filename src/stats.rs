use crate::codewars_requests::{get_completed, get_honor};
use crate::db::{ChatMessage, CodeUser, UserId};
use crate::error::MainError;
use futures::future::join_all;
use plotlib::style::BoxStyle;
use plotlib::{page, repr, view};
use resvg::usvg;
use std::collections::HashMap;
use std::iter::once;
use std::path::PathBuf;
use svg;
use uuid;

const SIZE_MULT: u32 = 2;
const SPACE_LEN: u32 = 40;

pub async fn compute_honor(users: HashMap<UserId, CodeUser>) -> Result<PathBuf, MainError> {
    let honors = join_all(users.values().cloned().map(|u: CodeUser| async {
        let u = u;
        Result::<_, MainError>::Ok((
            get_honor(u.codewars_name.as_str()).await?,
            u.firstname.to_owned(),
        ))
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    let maxy = honors.iter().map(|hn| hn.0).max().unwrap_or(50);
    let bars = honors
        .iter()
        .map(|(h, n)| {
            repr::BarChart::new(*h as f64)
                .label(n.to_owned())
                .style(&BoxStyle::new().fill("blue"))
        })
        .collect::<Vec<_>>();

    let mut view = view::CategoricalView::new()
        //.x_ticks(
        //    bars.iter()
        //        .map(|b| b.get_label().clone())
        //        .collect::<Vec<_>>()
        //        .as_slice(),
        //)
        .y_range(0., maxy as f64)
        .x_label("users")
        .y_label("honor");

    let width = bars
        .iter()
        .map(|bar| (bar.get_label().chars().count() as u32 + SPACE_LEN) * SIZE_MULT)
        .sum();

    for bar in bars {
        view = view.add(bar)
    }

    Ok(to_image(
        page::Page::single(&view).dimensions(600.max(width), 600),
    ))
}

pub async fn compute_stats(
    users: HashMap<UserId, CodeUser>,
    messages: Vec<ChatMessage>,
) -> Result<PathBuf, MainError> {
    let mut user_stats = Vec::new();
    let mut maxy = 5;
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
        user_stats.push((user.clone(), solved_in_scala.len(), sent_to_chat));

        maxy = maxy.max(solved_in_scala.len().max(sent_to_chat));
    }

    let bars: Vec<repr::BarChart> = user_stats
        .into_iter()
        .map(|(u, so, se)| {
            let label = u.firstname;
            once(
                repr::BarChart::new(so as f64)
                    .label(format!("{} solved", label))
                    .style(&BoxStyle::new().fill("orange")),
            )
            .chain(once(
                repr::BarChart::new(se as f64)
                    .label(format!("{} sent", label))
                    .style(&BoxStyle::new().fill("green")),
            ))
        })
        .flatten()
        .collect();

    let mut view = view::CategoricalView::new()
        //.x_ticks(
        //    bars.iter()
        //        .map(|b| b.get_label().clone())
        //        .collect::<Vec<_>>()
        //        .as_slice(),
        //)
        .y_range(0., maxy as f64)
        .x_label("users")
        .y_label("katas");

    let width = bars
        .iter()
        .map(|bar| (bar.get_label().chars().count() as u32 + SPACE_LEN) * SIZE_MULT)
        .sum();

    for bar in bars {
        view = view.add(bar)
    }
    Ok(to_image(
        page::Page::single(&view).dimensions(600.max(width), 600),
    ))
}

fn to_image(page: page::Page) -> PathBuf {
    let mut bytes = Vec::new();
    svg::write(&mut bytes, &page.to_svg().unwrap()).unwrap();
    let svg = usvg::Tree::from_data(
        bytes.as_slice(),
        &usvg::Options {
            font_family: "Liberation Serif".to_string(),
            ..usvg::Options::default()
        },
    )
    .unwrap();
    let mut img = resvg::default_backend()
        .render_to_image(&svg, &resvg::Options::default())
        .unwrap();
    let path = PathBuf::from(format!("tmp/img_{}.png", uuid::Uuid::new_v4()).as_str());

    // create dir if doesn't exist
    let mut dir_path = path.clone();
    dir_path.pop();
    if !dir_path.exists() {
        std::fs::create_dir_all(dir_path).unwrap();
    }

    img.save_png(path.as_path());
    path
}
