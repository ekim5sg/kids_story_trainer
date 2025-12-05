// E:\rust_dev\kids_story_trainer\src\main.rs
use gloo_net::http::Request;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlInputElement};
use yew::events::{InputEvent, MouseEvent};
use yew::prelude::*;
use yew::TargetCast;

// 🔗 Your deployed Cloudflare Worker URL
const WORKER_URL: &str =
    "https://kids-story-worker.mikegyver.workers.dev/api/story";

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct Story {
    title: String,
    paragraphs: Vec<String>,
    questions: Vec<Question>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum QuestionKind {
    MultipleChoice {
        choices: Vec<String>,
        correct_index: usize,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct Question {
    text: String,
    paragraph_index: usize,
    #[serde(flatten)]
    kind: QuestionKind,
}

#[derive(Clone, PartialEq)]
struct QuestionProgress {
    attempts: u32,
    is_correct: bool,
    skipped: bool,
}

#[derive(Clone, PartialEq)]
enum AppPhase {
    SelectTopic,
    LoadingStory,
    ReadStory,
    Questioning,
    Finished,
}

#[function_component(App)]
fn app() -> Html {
    let topic = use_state(|| "".to_string());
    let num_paragraphs = use_state(|| 3u8);
    let story = use_state(|| Option::<Story>::None);
    let question_progress = use_state(|| Vec::<QuestionProgress>::new());
    let current_question = use_state(|| 0usize);
    let phase = use_state(|| AppPhase::SelectTopic);
    let use_ai = use_state(|| false); // true if Cloudflare AI used
    let is_loading = use_state(|| false);
    let error = use_state(|| Option::<String>::None);

    // UI state for current answer (MC only)
    let selected_choice = use_state(|| Option::<usize>::None);

    // Input handlers
    let on_topic_input = {
        let topic = topic.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            topic.set(input.value());
        })
    };

    let on_paragraphs_input = {
        let num_paragraphs = num_paragraphs.clone();
        Callback::from(move |e: InputEvent| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(v) = input.value().parse::<u8>() {
                let v = v.clamp(1, 6);
                num_paragraphs.set(v);
            }
        })
    };

    let reset_quiz_state = {
        let question_progress = question_progress.clone();
        let current_question = current_question.clone();
        let selected_choice = selected_choice.clone();
        let phase = phase.clone();
        let error = error.clone();

        Callback::from(move |_| {
            error.set(None);
            selected_choice.set(None);
            current_question.set(0);
            question_progress.set(Vec::new());
            phase.set(AppPhase::ReadStory);
        })
    };

    // Generate story via Cloudflare Worker AI, with fallback to local stories
    let on_generate_story = {
        let topic = topic.clone();
        let num_paragraphs = num_paragraphs.clone();
        let story_state = story.clone();
        let question_progress = question_progress.clone();
        let current_question = current_question.clone();
        let phase = phase.clone();
        let use_ai = use_ai.clone();
        let is_loading = is_loading.clone();
        let error = error.clone();
        let selected_choice = selected_choice.clone();

        Callback::from(move |_| {
            let topic_value = (*topic).trim().to_string();
            if topic_value.is_empty() {
                error.set(Some("Please enter a story topic first.".into()));
                return;
            }

            error.set(None);
            is_loading.set(true);
            phase.set(AppPhase::LoadingStory);
            selected_choice.set(None);
            current_question.set(0);
            question_progress.set(Vec::new());

            let topic_for_async = topic_value.clone();
            let num_paragraphs_for_async = *num_paragraphs;
            let story_state = story_state.clone();
            let question_progress = question_progress.clone();
            let current_question = current_question.clone();
            let phase = phase.clone();
            let use_ai = use_ai.clone();
            let is_loading = is_loading.clone();
            let error = error.clone();

            spawn_local(async move {
                let payload = serde_json::json!({
                    "topic": topic_for_async,
                    "gradeLevel": 5,
                    "numParagraphs": num_paragraphs_for_async,
                    "numQuestions": 4
                });

                let mut error_msg: Option<String> = None;

                let story_res: Option<Story> =
                    match Request::post(WORKER_URL).json(&payload) {
                        Err(e) => {
                            error_msg = Some(format!(
                                "Could not build AI request; using fallback. ({})",
                                e
                            ));
                            None
                        }
                        Ok(req) => match req.send().await {
                            Ok(resp) if resp.status() == 200 => {
                                match resp.json::<Story>().await {
                                    Ok(st) => Some(st),
                                    Err(e) => {
                                        error_msg = Some(format!(
                                            "AI response parse error; using fallback. ({})",
                                            e
                                        ));
                                        None
                                    }
                                }
                            }
                            Ok(resp) => {
                                error_msg = Some(format!(
                                    "AI story request failed with status {}; using fallback.",
                                    resp.status()
                                ));
                                None
                            }
                            Err(e) => {
                                error_msg = Some(format!(
                                    "Could not reach AI Worker; using fallback. ({})",
                                    e
                                ));
                                None
                            }
                        },
                    };

                let final_story = if let Some(st) = story_res {
                    use_ai.set(true);
                    st
                } else {
                    use_ai.set(false);
                    pick_fallback_story(num_paragraphs_for_async)
                };

                if let Some(msg) = error_msg {
                    error.set(Some(msg));
                } else {
                    error.set(None);
                }

                let qp = final_story
                    .questions
                    .iter()
                    .map(|_| QuestionProgress {
                        attempts: 0,
                        is_correct: false,
                        skipped: false,
                    })
                    .collect::<Vec<_>>();

                story_state.set(Some(final_story));
                question_progress.set(qp);
                current_question.set(0);
                phase.set(AppPhase::ReadStory);
                is_loading.set(false);
            });
        })
    };

    // Child acknowledges reading full story; switch to questions
    let on_ack_read_story = {
        let phase = phase.clone();
        Callback::from(move |_| {
            phase.set(AppPhase::Questioning);
        })
    };

    // Handle radio choice selection
    let on_choice_change = {
        let selected_choice = selected_choice.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            if let Ok(idx) = input.value().parse::<usize>() {
                selected_choice.set(Some(idx));
            }
        })
    };

    // Check current answer (MC only), linear progression
    let on_check_answer = {
        let story = story.clone();
        let question_progress = question_progress.clone();
        let current_question = current_question.clone();
        let selected_choice = selected_choice.clone();
        let phase = phase.clone();
        let error = error.clone();

        Callback::from(move |_| {
            // Clear any old error first
            error.set(None);

            let Some(st) = (*story).clone() else {
                error.set(Some("No story is loaded yet.".into()));
                return;
            };

            let mut qp_vec = (*question_progress).clone();
            let q_index = *current_question;
            if q_index >= st.questions.len() {
                return;
            }

            // If this question is already done, ignore further clicks.
            let current_qp = &qp_vec[q_index];
            if current_qp.is_correct || current_qp.skipped {
                return;
            }

            let q = &st.questions[q_index];
            let selected = (*selected_choice).clone();

            // Don't count attempts if no choice is selected
            if selected.is_none() {
                error.set(Some("Please choose an answer before checking.".into()));
                return;
            }

            let is_correct = is_answer_correct(q, selected);

            let mut qp = qp_vec[q_index].clone();
            qp.attempts += 1;

            if is_correct {
                qp.is_correct = true;
                qp_vec[q_index] = qp;
                question_progress.set(qp_vec);

                // Move linearly to next question, or finish
                let next_index = q_index + 1;
                if next_index < st.questions.len() {
                    current_question.set(next_index);
                    selected_choice.set(None);
                    phase.set(AppPhase::Questioning);
                } else {
                    phase.set(AppPhase::Finished);
                }
            } else {
                // Mark attempt but stay on same question
                qp_vec[q_index] = qp;
                question_progress.set(qp_vec);
            }
        })
    };

    // Skip question (0 points), linear progression
    let on_skip_question = {
        let story = story.clone();
        let question_progress = question_progress.clone();
        let current_question = current_question.clone();
        let selected_choice = selected_choice.clone();
        let phase = phase.clone();

        Callback::from(move |_| {
            let Some(st) = (*story).clone() else {
                return;
            };

            let mut qp_vec = (*question_progress).clone();
            let idx = *current_question;
            if idx >= qp_vec.len() {
                return;
            }

            // Already done? Ignore.
            if qp_vec[idx].is_correct || qp_vec[idx].skipped {
                return;
            }

            let mut qp = qp_vec[idx].clone();
            qp.skipped = true;
            qp_vec[idx] = qp;
            question_progress.set(qp_vec);

            let next_index = idx + 1;
            if next_index < st.questions.len() {
                current_question.set(next_index);
                selected_choice.set(None);
                phase.set(AppPhase::Questioning);
            } else {
                phase.set(AppPhase::Finished);
            }
        })
    };

    // Retry the same story with fresh question progress
    let on_retry_story = {
        let question_progress = question_progress.clone();
        let current_question = current_question.clone();
        let selected_choice = selected_choice.clone();
        let phase = phase.clone();
        let error = error.clone();

        Callback::from(move |_| {
            // Clear any old error
            error.set(None);

            let old_qp = (*question_progress).clone();
            if old_qp.is_empty() {
                // No questions to retry; nothing to do
                return;
            }

            // Reset attempts / correctness / skipped for each question,
            // preserving the same length as the story's questions.
            let reset_vec = old_qp
                .iter()
                .map(|_| QuestionProgress {
                    attempts: 0,
                    is_correct: false,
                    skipped: false,
                })
                .collect::<Vec<_>>();

            question_progress.set(reset_vec);
            current_question.set(0);
            selected_choice.set(None);
            phase.set(AppPhase::Questioning);
        })
    };

    // Start over from topic selection
    let on_restart = {
        let topic = topic.clone();
        let num_paragraphs = num_paragraphs.clone();
        let story = story.clone();
        let question_progress = question_progress.clone();
        let current_question = current_question.clone();
        let phase = phase.clone();
        let use_ai = use_ai.clone();
        let selected_choice = selected_choice.clone();
        let error = error.clone();

        Callback::from(move |_| {
            topic.set("".into());
            num_paragraphs.set(3);
            story.set(None);
            question_progress.set(Vec::new());
            current_question.set(0);
            phase.set(AppPhase::SelectTopic);
            use_ai.set(false);
            selected_choice.set(None);
            error.set(None);
        })
    };

    // Compute score and letter grade
    let (score_percent, grade_label) = {
        let st_opt = (*story).clone();
        let qp_vec = (*question_progress).clone();
        if let Some(st) = st_opt {
            if st.questions.is_empty() || qp_vec.is_empty() {
                (None, None)
            } else {
                let total = st.questions.len() as f32;
                let points_per = 100.0 / total;

                let mut score = 0.0;
                for qprog in qp_vec.iter() {
                    if qprog.skipped {
                        continue;
                    }
                    if qprog.is_correct {
                        score += points_per;
                    }
                }

                let score_rounded = score.round() as i32;
                let grade = if score_rounded >= 90 {
                    ("A".to_string(), "Excellent".to_string())
                } else if score_rounded >= 80 {
                    ("B".to_string(), "Good".to_string())
                } else if score_rounded >= 70 {
                    ("C".to_string(), "Needs Practice".to_string())
                } else {
                    ("Unsatisfactory".to_string(), "Keep Working!".to_string())
                };
                (Some(score_rounded), Some(grade))
            }
        } else {
            (None, None)
        }
    };

    html! {
        <div class="app-shell">
            <header>
                <h1>{"Kids Story Trainer (5th Grade)"}</h1>
                <p class="sub">
                    {"Pick a topic, let the app (or Cloudflare AI) write a story, then practice comprehension with multiple-choice questions."}
                </p>
                <div>
                    <span class="pill tag-ai">{"Cloudflare Worker AI (primary)"}</span>
                    <span class="pill tag-fallback">{"Built-in stories (fallback)"}</span>
                </div>
            </header>

            <main>
                <section>
                    <h2>{"1. Choose a topic & story size"}</h2>
                    <div class="row">
                        <div>
                            <label>{"Story topic (kid friendly)"}</label>
                            <input
                                type="text"
                                placeholder="Volcano safety, a school field trip, a science fair, a lost puppy..."
                                value={(*topic).clone()}
                                oninput={on_topic_input}
                            />
                        </div>
                        <div style="max-width: 200px;">
                            <label>{"Number of paragraphs"}</label>
                            <input
                                type="number"
                                min="1"
                                max="6"
                                value={num_paragraphs.to_string()}
                                oninput={on_paragraphs_input}
                            />
                            <p class="sub">{"Usually 2–5 works well for 5th grade."}</p>
                        </div>
                    </div>
                    <button class="btn btn-primary" onclick={on_generate_story} disabled={*is_loading}>
                        { if *is_loading { "Generating story..." } else { "Generate Story & Questions" } }
                    </button>
                    if let Some(err) = &*error {
                        <div class="error">
                            {err}
                        </div>
                    }
                </section>

                {
                    match &*phase {
                        AppPhase::SelectTopic | AppPhase::LoadingStory => html! {},
                        _ => {
                            if let Some(st) = &*story {
                                html! {
                                    <>
                                        <h2>{"2. Read the story"}</h2>
                                        <div class="story-box">
                                            <h3>{ &st.title }</h3>
                                            {
                                                for st.paragraphs.iter().enumerate().map(|(i, p)| {
                                                    html! {
                                                        <div class="paragraph">
                                                            <strong>{"Paragraph "}{ i + 1 }{":"}</strong>
                                                            <br />
                                                            { p }
                                                        </div>
                                                    }
                                                })
                                            }
                                        </div>
                                        <button class="btn btn-secondary" onclick={reset_quiz_state.clone()}>
                                            {"Back to this story"}
                                        </button>
                                        <button class="btn btn-primary" onclick={on_ack_read_story.clone()}>
                                            {"I read the story – start questions"}
                                        </button>
                                    </>
                                }
                            } else {
                                html! {}
                            }
                        }
                    }
                }

                {
                    match &*phase {
                        AppPhase::Questioning => render_question_ui(
                            &story,
                            &question_progress,
                            &current_question,
                            &selected_choice,
                            &on_choice_change,
                            &on_check_answer,
                            &on_skip_question,
                        ),
                        AppPhase::Finished => render_results_ui(
                            &story,
                            &question_progress,
                            &score_percent,
                            &grade_label,
                            &on_restart,
                            &on_retry_story,
                        ),
                        _ => html! {},
                    }
                }
            </main>

            <footer class="footer">
                <span>
                    {"v0.7.2 – Rust + Yew + WASM · MC-only"}
                    { if *use_ai { " · Cloudflare AI" } else { " · Fallback story" } }
                </span>
            </footer>
        </div>
    }
}

// --- Helper rendering functions -------------------------------------------------

fn render_question_ui(
    story: &UseStateHandle<Option<Story>>,
    question_progress: &UseStateHandle<Vec<QuestionProgress>>,
    current_question: &UseStateHandle<usize>,
    selected_choice: &UseStateHandle<Option<usize>>,
    on_choice_change: &Callback<Event>,
    on_check_answer: &Callback<MouseEvent>,
    on_skip_question: &Callback<MouseEvent>,
) -> Html {
    let Some(st) = (**story).clone() else {
        return html! {};
    };
    let qp_vec = &**question_progress;
    if st.questions.is_empty() || qp_vec.is_empty() {
        return html! {};
    }

    let idx = **current_question;
    if idx >= st.questions.len() {
        return html! {};
    }
    let q = &st.questions[idx];
    let qp = &qp_vec[idx];

    let total = st.questions.len();
    let answered = qp_vec.iter().filter(|q| q.is_correct || q.skipped).count();

    let is_done = qp.is_correct || qp.skipped;

    // For display: clamp weird values so "1 attempt" displays as 1 when correct
    let display_attempts = if qp.is_correct && qp.attempts > 1 {
        1
    } else {
        qp.attempts
    };

    // Status message: simple + truthful
    let status_msg: &str = if qp.is_correct {
        " · ✅ Correct!"
    } else if qp.skipped {
        " · This question was skipped (0 points)."
    } else {
        ""
    };

    html! {
        <section>
            <h2>{"3. Answer comprehension questions"}</h2>
            <div class="question-box">
                <p class="sub">
                    {"Question "}{ idx + 1 }{" of "}{ total }
                    {" · "}
                    {"Completed: "}{ answered }{"/"}{ total }
                </p>
                <p><strong>{"Q: "}{ &q.text }</strong></p>

                {
                    match &q.kind {
                        QuestionKind::MultipleChoice { choices, .. } => {
                            html! {
                                <div class="choices">
                                    {
                                        for choices.iter().enumerate().map(|(i, choice)| {
                                            let value = i.to_string();
                                            let checked = (*selected_choice).map(|v| v == i).unwrap_or(false);
                                            html! {
                                                <label class="choice">
                                                    <input
                                                        type="radio"
                                                        name="mc-choice"
                                                        value={value}
                                                        checked={checked}
                                                        onchange={on_choice_change.clone()}
                                                        disabled={is_done}
                                                    />
                                                    { choice }
                                                </label>
                                            }
                                        })
                                    }
                                </div>
                            }
                        }
                    }
                }

                <div>
                    <button class="btn btn-primary" onclick={on_check_answer.clone()} disabled={is_done}>
                        {"Check Answer"}
                    </button>
                    <button class="btn btn-secondary" onclick={on_skip_question.clone()} disabled={is_done}>
                        {"Skip (0 pts)"}
                    </button>
                </div>

                <div class="status-line">
                    <strong>{"Attempts: "}{ display_attempts }</strong>
                    { status_msg }
                </div>
            </div>
        </section>
    }
}

fn render_results_ui(
    story: &UseStateHandle<Option<Story>>,
    question_progress: &UseStateHandle<Vec<QuestionProgress>>,
    score_percent: &Option<i32>,
    grade_label: &Option<(String, String)>,
    on_restart: &Callback<MouseEvent>,
    on_retry_story: &Callback<MouseEvent>,
) -> Html {
    let Some(st) = (**story).clone() else {
        return html! {};
    };
    let qp_vec = &**question_progress;

    let (grade_str, grade_desc, grade_class) = if let (Some(score), Some((grade, desc))) =
        (score_percent.clone(), grade_label.clone())
    {
        let badge_class = match grade.as_str() {
            "A" => "badge badge-a",
            "B" => "badge badge-b",
            "C" => "badge badge-c",
            _ => "badge badge-u",
        };
        (format!("{} ({}%)", grade, score), desc, badge_class.to_string())
    } else {
        (
            "No score".to_string(),
            "Try generating a story first.".to_string(),
            "badge badge-u".to_string(),
        )
    };

    // Allow retry for any completed quiz with score < 100
    let allow_retry = if let Some(score) = score_percent {
        *score < 100
    } else {
        false
    };

    html! {
        <section>
            <h2>{"4. Results"}</h2>
            <div class="question-box">
                <p>{"Story: "}<strong>{ &st.title }</strong></p>
                <p>
                    {"Grade: "}
                    <span class={grade_class}>{ grade_str }</span>
                    {" · "}{ grade_desc }
                </p>
                <ul>
                    {
                        for st.questions.iter().enumerate().map(|(i, _q)| {
                            let qp = &qp_vec[i];

                            // UI guard: if it's correct but attempts somehow >1, show 1
                            let display_attempts = if qp.is_correct && qp.attempts > 1 {
                                1
                            } else {
                                qp.attempts
                            };

                            let status = if qp.skipped {
                                "Skipped (0 pts)"
                            } else if qp.is_correct {
                                "Correct"
                            } else {
                                "Incomplete"
                            };
                            html! {
                                <li>
                                    {"Q"}{ i + 1 }{": "}{ status }
                                    {" · attempts: "}{ display_attempts }
                                </li>
                            }
                        })
                    }
                </ul>

                {
                    if allow_retry {
                        html! {
                            <button class="btn btn-secondary" onclick={on_retry_story.clone()}>
                                {"Retry this story"}
                            </button>
                        }
                    } else {
                        html! {}
                    }
                }

                <button class="btn btn-primary" onclick={on_restart.clone()}>
                    {"Start a new story"}
                </button>
            </div>
        </section>
    }
}

// --- Logic helpers ------------------------------------------------------------

fn is_answer_correct(q: &Question, selected_choice: Option<usize>) -> bool {
    match &q.kind {
        QuestionKind::MultipleChoice {
            correct_index, ..
        } => selected_choice.map(|i| i == *correct_index).unwrap_or(false),
    }
}

// --- Fallback stories ---------------------------------------------------------

fn pick_fallback_story(num_paragraphs: u8) -> Story {
    let mut rng = thread_rng();
    let mut stories = fallback_stories();
    stories.shuffle(&mut rng);
    let mut story = stories.remove(0);

    let desired = num_paragraphs.clamp(1, 6) as usize;
    if story.paragraphs.len() > desired {
        story.paragraphs.truncate(desired);
    }

    story
}

fn fallback_stories() -> Vec<Story> {
    vec![
        Story {
            title: "The Science Fair Mystery".into(),
            paragraphs: vec![
                "Maya loved science more than anything. When her school announced a science fair, she decided to build a tiny wind turbine that could power a small light bulb.".into(),
                "For weeks, she tested different blade shapes in front of a fan. Some blades barely moved, but others spun so fast that the bulb flickered to life.".into(),
                "On the day of the fair, Maya discovered that her project table had been bumped, and her blades were scattered on the floor. She stayed calm, rebuilt the turbine, and showed the judges how testing and patience helped her design improve.".into(),
            ],
            questions: vec![
                Question {
                    text: "What was Maya building for the science fair?".into(),
                    paragraph_index: 0,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "A robot that could talk".into(),
                            "A tiny wind turbine".into(),
                            "A solar-powered car".into(),
                            "A model of the solar system".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "Where did Maya test her different blade shapes?".into(),
                    paragraph_index: 1,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "In a swimming pool".into(),
                            "In front of a fan".into(),
                            "On the school roof".into(),
                            "In the gym".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "How did Maya react when she saw her blades on the floor?".into(),
                    paragraph_index: 2,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "She shouted at her classmates".into(),
                            "She went home and quit the fair".into(),
                            "She stayed calm and rebuilt the turbine".into(),
                            "She asked the judges to skip her project".into(),
                        ],
                        correct_index: 2,
                    },
                },
            ],
        },
        Story {
            title: "The Lost Backpack on the Bus".into(),
            paragraphs: vec![
                "Jamal always double-checked his backpack before leaving school. One rainy afternoon, he rushed to catch the bus and forgot to zip it closed.".into(),
                "On the ride home, the bus bumped over a pothole. Jamal’s notebook slid out of his open backpack and under the seat without him noticing.".into(),
                "When he got home, Jamal realized his notebook was missing. He thought carefully about his day and remembered the bump on the bus, so he called the bus driver and they found the notebook under the seat.".into(),
            ],
            questions: vec![
                Question {
                    text: "What did Jamal forget to do before he got on the bus?".into(),
                    paragraph_index: 0,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "Put on his shoes".into(),
                            "Zip his backpack".into(),
                            "Finish his homework".into(),
                            "Call his friend".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "Where did the notebook go when the bus hit the pothole?".into(),
                    paragraph_index: 1,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "Out the window".into(),
                            "Into another student’s backpack".into(),
                            "Under the seat".into(),
                            "Onto the driver’s chair".into(),
                        ],
                        correct_index: 2,
                    },
                },
                Question {
                    text: "How did Jamal finally find his notebook?".into(),
                    paragraph_index: 2,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "He searched the school hallway".into(),
                            "He called the bus driver".into(),
                            "His teacher brought it home".into(),
                            "A friend mailed it to him".into(),
                        ],
                        correct_index: 1,
                    },
                },
            ],
        },
        Story {
            title: "The Classroom Garden".into(),
            paragraphs: vec![
                "Ms. Lopez brought small pots, soil, and seeds to class. She told her students they would grow a mini garden on the windowsill.".into(),
                "Each student planted a seed and wrote their name on the pot. Some seeds sprouted quickly, while others took more time to peek through the soil.".into(),
                "When one student’s seed did not sprout, the class worked together to check the soil, water, and sunlight. They planted a new seed, and the student learned that plants sometimes need a second chance too.".into(),
            ],
            questions: vec![
                Question {
                    text: "Where did the class keep their mini garden?".into(),
                    paragraph_index: 0,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "On the playground".into(),
                            "In the gym".into(),
                            "On the windowsill".into(),
                            "In the cafeteria".into(),
                        ],
                        correct_index: 2,
                    },
                },
                Question {
                    text: "What did each student write on their pot?".into(),
                    paragraph_index: 1,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "A science question".into(),
                            "A funny joke".into(),
                            "Their favorite color".into(),
                            "Their name".into(),
                        ],
                        correct_index: 3,
                    },
                },
                Question {
                    text: "What did the class do when one seed did not sprout?".into(),
                    paragraph_index: 2,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "They threw the pot away".into(),
                            "They ignored it".into(),
                            "They checked the plant’s needs and tried again".into(),
                            "They stopped watering all the plants".into(),
                        ],
                        correct_index: 2,
                    },
                },
            ],
        },
        Story {
            title: "The Library Map Challenge".into(),
            paragraphs: vec![
                "The school librarian, Mr. Lee, created a map of the library with clues. He told the class they would use the map to find a hidden box of bookmarks.".into(),
                "The map showed different sections, like history, science, and sports. Each clue led to a new shelf and taught the students how books were organized.".into(),
                "When the class finally found the hidden box, Mr. Lee explained that learning to read maps could help them explore both books and the real world.".into(),
            ],
            questions: vec![
                Question {
                    text: "What did the map in the library lead to?".into(),
                    paragraph_index: 0,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "A secret doorway".into(),
                            "A hidden box of bookmarks".into(),
                            "A new computer lab".into(),
                            "A stack of comic books".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "Which section was mentioned on the library map?".into(),
                    paragraph_index: 1,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "Weather".into(),
                            "History".into(),
                            "Cooking".into(),
                            "Music videos".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "What did Mr. Lee want students to learn from the map challenge?".into(),
                    paragraph_index: 2,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "How to whisper quietly".into(),
                            "How to walk faster".into(),
                            "How to read maps and explore".into(),
                            "How to put books on the floor".into(),
                        ],
                        correct_index: 2,
                    },
                },
            ],
        },
        Story {
            title: "The Rainy Day Coding Club".into(),
            paragraphs: vec![
                "On a rainy Friday, the after-school coding club met in the computer lab. Their challenge was to program a character to move through a simple maze.".into(),
                "At first, the character kept bumping into walls. The students tested different commands, like turn, move forward, and repeat, until the character reached the goal.".into(),
                "By the end of the club, the students realized that fixing mistakes was a normal part of coding, and each error had helped them understand the maze better.".into(),
            ],
            questions: vec![
                Question {
                    text: "What was the challenge at the coding club?".into(),
                    paragraph_index: 0,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "Build a robot dog".into(),
                            "Program a character to move through a maze".into(),
                            "Design a new video game console".into(),
                            "Write a story about coding".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "Which type of commands did the students test?".into(),
                    paragraph_index: 1,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "Sing and dance".into(),
                            "Turn, move forward, and repeat".into(),
                            "Jump and spin".into(),
                            "Erase and redraw".into(),
                        ],
                        correct_index: 1,
                    },
                },
                Question {
                    text: "What did the students learn about mistakes in coding?".into(),
                    paragraph_index: 2,
                    kind: QuestionKind::MultipleChoice {
                        choices: vec![
                            "Mistakes mean you should quit".into(),
                            "Mistakes are normal and help you learn".into(),
                            "Only teachers can fix mistakes".into(),
                            "Mistakes always break the computer".into(),
                        ],
                        correct_index: 1,
                    },
                },
            ],
        },
    ]
}

// -----------------------------------------------------------------------------

fn main() {
    yew::Renderer::<App>::new().render();
}