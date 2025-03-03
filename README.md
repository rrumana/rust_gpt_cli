# rust_gpt_cli
A CLI tool to interface with the ChatGPT API from the command line.

I built this to be able to use ChatGPT from my preffered CLI. It allows you to have relatively coherent conversations for about 20-30 mesages before the the context window gets a little screw-y. All that is needed is an OpenAI API Key and you're off to the races. A short Demo and better instructions on the way.

## Installation

```bash
git clone git@github.com:rrumana/rust_gpt_cli.git
cd rust_gpt_cli
cargo install --path .
```

make sure you have your OpenAI API key set in your environment variables as `OPENAI_API_KEY`

```bash
export OPENAI_API_KEY=your_api_key
```

## Usage

you can simply call `gpt` from the command line and start chatting with ChatGPT-4o

```bash
$ gpt
Interactive Chat Session (model: gpt-4o). Type your message below. Press Ctrl+C to exit.

hello chat gpt!
gpt-4o: Hello! How can I assist you today?

I am working on a top secret project!
gpt-4o: That sounds exciting! Feel free to let me know if there’s anything specific you need help with or if there’s anything I can do to support your project!

What was the first thing that I said to you?
gpt-4o: The first thing you said was "hello chat gpt!"

^C
Termination signal received.
Session ended. Press enter to exit.
```

you can also specify the model you want to use with the `--model` flag
Chain of throught is not displayed even when using reasoning models like o1 and o3-mini
```bash
$ gpt --model o3-mini
Interactive Chat Session (model: o3-mini). Type your message below. Press Ctrl+C to exit.

Think really hard about this... how many rs are in the word strawberry?
03-mini: The word "strawberry" contains three 'r's.
```

