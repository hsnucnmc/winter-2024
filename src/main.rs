use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use axum::response::Response;
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use rand::{thread_rng, Rng};
use std::time::Instant;
use usvg::{fontdb, Tree, TreeParsing, TreePostProc};

type QuestionID = u32;
type AnswerNum = u32;

#[derive(Default)]
struct AppState {
    active_qid: BTreeMap<QuestionID, AnswerNum>,
    qid_time: BTreeMap<Instant, QuestionID>,
    fontdb: fontdb::Database,
}

type SharedState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mut state = AppState::default();
    state.fontdb.load_system_fonts();

    let shared_state = SharedState::new(RwLock::new(state));

    let app = Router::new()
        .route("/new-qid", get(new_qid))
        .route("/captcha-img/:id", get(random_img))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("localhost:10069")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn random_img(State(state): State<SharedState>, Path(qid): Path<String>) -> Response {
    let state = state.read().unwrap();
    let qid: QuestionID = match QuestionID::from_str_radix(&qid, 16) {
        Ok(qid) => qid,
        Err(_) => return "".into_response(),
    };
    let ans = match state.active_qid.get(&qid) {
        Some(ans) => ans,
        None => return "".into_response(),
    };
    (
        [(axum::http::header::CONTENT_TYPE, "image/png")],
        get_png(*ans, &state.fontdb),
    )
        .into_response()
}

async fn new_qid(State(state): State<SharedState>) -> String {
    let (qid, ans) = loop {
        let qid: QuestionID = thread_rng().gen_range(0x00000000..=0x99999999);
        let ans: AnswerNum = thread_rng().gen_range(00000..=99999);

        let state = state.read().unwrap();
        if state.active_qid.contains_key(&qid) {
            continue;
        }

        break (qid, ans);
    };
    let mut state = state.write().unwrap();
    state.active_qid.insert(qid, ans);
    println!("{} {}", qid, ans);
    format!("{:08X}", qid)
}

fn get_png(mut ans: AnswerNum, fontdb: &fontdb::Database) -> Vec<u8> {
    let answer_digits = {
        assert!(ans <= 99999);

        let mut arr = [0; 5];
        for i in (0..5).rev() {
            arr[i] = ans % 10;
            ans /= 10;
        }
        arr
    };
    let svg_str = format!(
        r###"<svg xmlns='http://www.w3.org/2000/svg' height='100' width='280'>
    <text font-family='Ani' font-size='50' x='40' y='75' fill='{}' transform='rotate({})'>{}</text>
    <text font-family='Ani' font-size='50' x='80' y='75' fill='{}' transform='rotate({})'>{}</text>
    <text font-family='Ani' font-size='50' x='120' y='75' fill='{}' transform='rotate({})'>{}</text>
    <text font-family='Ani' font-size='50' x='160' y='75' fill='{}' transform='rotate({})'>{}</text>
    <text font-family='Ani' font-size='50' x='200' y='75' fill='{}' transform='rotate({})'>{}</text>
</svg>"###,
        random_color(),
        random_rotation(),
        answer_digits[0],
        random_color(),
        random_rotation(),
        answer_digits[1],
        random_color(),
        random_rotation(),
        answer_digits[2],
        random_color(),
        random_rotation(),
        answer_digits[3],
        random_color(),
        random_rotation(),
        answer_digits[4]
    );
    let mut tree = Tree::from_str(&svg_str, &usvg::Options::default()).unwrap();
    let steps = usvg::PostProcessingSteps {
        convert_text_into_paths: true,
    };
    tree.postprocess(steps, &fontdb);

    let pixmap_size = tree.size.to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(pixmap_size.width(), pixmap_size.height()).unwrap();
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().unwrap()
}

fn random_color() -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        rand::thread_rng().gen_range(0x22..=0xCC),
        rand::thread_rng().gen_range(0x22..=0xCC),
        rand::thread_rng().gen_range(0x22..=0xCC)
    )
}
fn random_digit() -> u32 {
    rand::thread_rng().gen_range(0..=9)
}

fn random_rotation() -> f32 {
    rand::thread_rng().gen_range(-8f32..=8f32)
}
