use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dashmap::DashSet;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use std::time::{Duration, Instant};
use usvg::{fontdb, Tree, TreeParsing, TreePostProc};

type QuestionID = u32;
type AnswerNum = u32;

#[derive(Debug, PartialEq, Eq)]
enum QuestionState {
    NewlyGenerated,
    WaitingAnswer,
}

#[derive(Default)]
struct AppState {
    qid_answer: BTreeMap<QuestionID, AnswerNum>,
    qid_state: BTreeMap<QuestionID, QuestionState>,
    qid_time: BTreeMap<QuestionID, Instant>,
    time_qid_qid: BTreeMap<(Instant, QuestionID), QuestionID>,
    fontdb: fontdb::Database,
    users: DashSet<String>,
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
        .route("/captcha-img/:id", get(captcha_img))
        .route("/submit", post(submit))
        .route("/users", get(users))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("localhost:10069")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn new_qid(State(state): State<SharedState>) -> String {
    let (qid, ans) = loop {
        let qid: QuestionID = thread_rng().gen_range(0x00000000..=0x99999999);
        let ans: AnswerNum = thread_rng().gen_range(00000..=99999);

        let state = state.read().unwrap();
        if state.qid_answer.contains_key(&qid) {
            continue;
        }

        break (qid, ans);
    };
    let mut state = state.write().unwrap();
    state.qid_answer.insert(qid, ans);
    state.qid_state.insert(qid, QuestionState::NewlyGenerated);
    let now = Instant::now();
    state.qid_time.insert(qid, now);
    state.time_qid_qid.insert((now, qid), qid);
    println!("{} {}", qid, ans);
    format!("{}", qid)
}

async fn captcha_img(State(state): State<SharedState>, Path(qid): Path<String>) -> Response {
    let qid: QuestionID = match QuestionID::from_str_radix(&qid, 10) {
        Ok(qid) => qid,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if state.read().unwrap().qid_state.get(&qid) != Some(&QuestionState::NewlyGenerated) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let ans = {
        let mut state = state.write().unwrap();
        state.qid_state.insert(qid, QuestionState::WaitingAnswer);
        state.qid_answer[&qid]
    };

    (
        [(header::CONTENT_TYPE, "image/png")],
        generate_image(ans, &state.read().unwrap().fontdb),
    )
        .into_response()
}

async fn submit(
    State(state): State<SharedState>,
    Json(CreateUser {
        username,
        captcha_qid,
        captcha_ans,
    }): Json<CreateUser>,
) -> Response {
    let captcha_ans: AnswerNum = match captcha_ans.parse() {
        Ok(ans) => ans,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    if state.read().unwrap().qid_state.get(&captcha_qid) != Some(&QuestionState::WaitingAnswer) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if Instant::now() - state.read().unwrap().qid_time[&captcha_qid] > Duration::from_secs(3600) {
        let mut state = state.write().unwrap();
        assert!(state.qid_answer.remove(&captcha_qid).is_some());
        assert!(state.qid_state.remove(&captcha_qid).is_some());
        let time = state.qid_time.remove(&captcha_qid).unwrap();
        assert!(state.time_qid_qid.remove(&(time, captcha_qid)).is_some());
        return StatusCode::UNAUTHORIZED.into_response();
    }

    if state.read().unwrap().qid_answer[&captcha_qid] != captcha_ans {
        let mut state = state.write().unwrap();
        assert!(state.qid_answer.remove(&captcha_qid).is_some());
        assert!(state.qid_state.remove(&captcha_qid).is_some());
        let time = state.qid_time.remove(&captcha_qid).unwrap();
        assert!(state.time_qid_qid.remove(&(time, captcha_qid)).is_some());
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mut state = state.write().unwrap();
    assert!(state.qid_answer.remove(&captcha_qid).is_some());
    assert!(state.qid_state.remove(&captcha_qid).is_some());
    let time = state.qid_time.remove(&captcha_qid).unwrap();
    assert!(state.time_qid_qid.remove(&(time, captcha_qid)).is_some());

    if username.split_whitespace().count() > 1 {
        return StatusCode::BAD_REQUEST.into_response();
    }

    println!("Created user: {}", &username);
    state.users.insert(username);

    StatusCode::CREATED.into_response()
}

async fn users(State(state): State<SharedState>) -> Response {
    let mut user_list = String::new();
    for username in state.read().unwrap().users.iter() {
        user_list.push_str(&username);
        user_list.push('\n');
    }

    user_list.into_response()
}

fn generate_image(mut ans: AnswerNum, fontdb: &fontdb::Database) -> Vec<u8> {
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

fn random_rotation() -> f32 {
    rand::thread_rng().gen_range(-8f32..=8f32)
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
    captcha_qid: QuestionID,
    captcha_ans: String,
}
