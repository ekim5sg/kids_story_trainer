# kids_story_trainer
📚 Kids Story Trainer

A Rust + WebAssembly + Cloudflare AI Reading Comprehension Generator

Built to help students practice literacy and comprehension using fun, personalized reading passages — powered by Cloudflare Workers AI and delivered through a clean, distraction-free Web UI.

🚀 Overview

Kids Story Trainer lets a learner:

🧠 Pick any topic (from dinosaurs to bubble tea)

✨ Generate a custom story using Cloudflare Worker AI

✔️ Read the passage

🎯 Answer comprehension questions

📈 Track attempts, improvement, and accuracy

It’s designed for:

👦 Students in grades 2–6

👨‍🏫 Educators exploring adaptive EdTech

🏫 Districts piloting AI-assisted learning

👨‍👩‍👦 Parents supporting reading confidence

🧪 Developers who want a practical Rust + WASM + Cloudflare example

🧰 Tech Stack
Layer	Technology
Frontend	🦀 Rust + Yew (compiled to WebAssembly)
Backend AI	☁️ Cloudflare Worker AI (text-generation + JSON responses)
Networking	gloo-net
Randomization	rand crate
Build Tooling	trunk + wasm-bindgen
Deployment Options	Cloudflare Pages, static hosting such as Hostek, GitHub Pages
📦 Features

✔️ Cloudflare Worker generates:

A title

Custom story paragraphs

Multiple choice questions

Correct answer tracking

✔️ Built-in fallback stories if offline or AI unavailable
✔️ Tracks attempts per question (no answer = no attempt counted)
✔️ Prevents skipping forward until answered or intentionally skipped
✔️ Retry system — student can replay the same story
✔️ Mobile-friendly UI
✔️ Safe for school — no logins, no data retention, no tracking

🏗️ Setup & Development
1️⃣ Install Rust toolchain
rustup update
rustup target add wasm32-unknown-unknown

2️⃣ Install Trunk
cargo install trunk

3️⃣ Clone the Repo
git clone https://github.com/<your-repo>/kids_story_trainer
cd kids_story_trainer

4️⃣ Run Development Server
trunk serve --open

5️⃣ Build for Deployment
trunk build --release


Output will be located in:

/dist


Upload this folder to your static hosting provider (Cloudflare Pages, Hostek, Netlify, GitHub Pages, etc).

☁️ Cloudflare Worker Setup

Create a Worker:

wrangler init kids-story-worker


Then ensure your Worker has:

@cloudflare/ai bindings

/api/story POST route returning JSON in the format:

{
  "title":"Example Story",
  "paragraphs":["..."],
  "questions":[
      {
         "text":"What happened in the story?",
         "paragraph_index":0,
         "kind":"multiple_choice",
         "choices":["Correct","Wrong","Wrong","Wrong"],
         "correct_index":0
      }
  ]
}


Add the Worker URL to the Yew app:

const WORKER_URL: &str = "https://your-worker-url.workers.dev/api/story";

🧪 Testing Checklist
Behavior	Status
AI story loads successfully with valid topic	✔️
Fallback story triggers on offline/500 error	✔️
Attempts count only when answer is submitted	✔️
Correct lockout prevents re-answering	✔️
Final score and retry option work	✔️
Works on iPad, Chromebook, and desktop	✔️
🌱 Roadmap / Future Enhancements

🔊 Text-to-speech narration

🎖️ Badge + reward system

🌍 Multilingual support (Spanish, Filipino, French)

🏷️ Teacher mode with downloadable analytics

🧩 Adaptive difficulty based on accuracy

🎨 Dark mode + dyslexia-friendly font option

👥 Contributors

Originally inspired by a conversation between:

👨‍💻 Michael Givens (Developer & Parent)
💡 Teenage brother (High school sophomore who suggested making it for SAT prep)
🎉 7-year-old user + product tester

📝 License

MIT License — fully open for families, teachers, and developers to build upon.

🙌 Want to Help?

If you're a:

Teacher interested in classroom testing

Developer who wants to add features

District IT leader exploring EdTech

Parent looking for offline literacy tools

📩 Please open an issue, submit a PR, or reach out.

⭐ If this helps a young learner — consider giving the repo a star!
