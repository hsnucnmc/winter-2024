use axum::{response::IntoResponse, routing::get, Router};
use rand::Rng;
use usvg::{fontdb, Tree, TreeParsing, TreePostProc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/random-img", get(random_img));
    let listener = tokio::net::TcpListener::bind("localhost:10069").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn random_img() -> impl IntoResponse {
    ([(axum::http::header::CONTENT_TYPE, "image/png")], get_png())
}

fn get_png() -> Vec<u8> {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    let (answer_digits, answer) = {
        let mut arr = [0; 5];
        let mut ans = 0;
        for i in 0..5 {
            arr[i] = random_digit();
            ans*=10;
            ans+=arr[i];
        }
        (arr, ans)
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
    println!("ans: {}", answer);
    pixmap.encode_png().unwrap()
}

fn random_color() -> String {
    format!("#{:02X}{:02X}{:02X}", rand::thread_rng().gen_range(0x22..=0xCC), rand::thread_rng().gen_range(0x22..=0xCC), rand::thread_rng().gen_range(0x22..=0xCC))
}
fn random_digit() -> u32 {
    rand::thread_rng().gen_range(0..=9)
}

fn random_rotation() -> f32 {
    rand::thread_rng().gen_range(-8f32..=8f32)
}
