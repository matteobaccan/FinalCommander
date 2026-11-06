# What the AI could do next — a study

*Written for 0.38.0. No code: this is the list to choose from, with what each idea costs and
what it sends. Nothing here is decided until the author picks it.*

## Where it stands

`ai: question` on the command line asks one of the `[[ai]]` services — anything that speaks
OpenAI's chat API, Anthropic's messages, or Ollama on the same machine — and opens the answer
in the viewer as Markdown (SPEC 06 §14). Only the question is sent, unless files are
**selected**, and then a dialog says how many, how large and to which host before anything
goes. The key is in an environment variable or the encrypted secrets file. Each answer is a
file in `fcmd-ai/`, so the history is a folder. *Commands → AI services* manages the services.

What it does not do: hold a conversation, stream, know what is on the screen beyond the
selected files, or produce anything the program can *use* — an answer is always prose to read.

## The rules every idea below keeps

These are the ones the present feature already keeps; any new use of the model that breaks one
of them is not worth its convenience.

1. **Nothing is sent that was not shown first.** What goes, how big, and to which host, in a
   dialog with *Cancel* lit — or, for the small things (a question, a file name), on the line
   where it is typed. Names and sizes count as content: a folder listing says a lot.
2. **Nothing is done that was not shown first.** A model proposes; the program shows the
   proposal in the place that already has a preview and a confirmation — the command line,
   the multi-rename dialog, the diff, the commit prompt — and the user's key does it.
3. **A local model is a first-class answer.** Every idea must work against Ollama with no key
   and no network, even if a large hosted model does it better.
4. **The answer is a file.** Whatever is asked and answered lands in `fcmd-ai/`, readable
   later, deletable, never sent anywhere again on its own.
5. **No background calls.** Nothing asks the model because the cursor moved.

## The ideas

Each with what it is, what is sent, where it lives in the program, and a rough size: **S** is
a day or less, **M** a few days, **L** a week or more.

### A. Making what exists better

| # | Idea | What is sent | Where | Size |
| --- | --- | --- | --- | --- |
| A1 | **Follow-up questions**: `Shift+Enter` (or `ai:` again in the answer's viewer) asks the next question in the same thread; the answer file grows a section per turn | the earlier turns of that thread, plus the new question | the viewer showing an answer | S |
| A2 | **Streaming**: the answer appears in the viewer as it arrives, `Esc` stops it | nothing new | the worker and the viewer | M |
| A3 | **Cost on the status line**: tokens in and out, as the services report them, and a per-service cap that refuses before sending | nothing new | the status line, `[[ai]]` | S |
| A4 | **Redaction before sending**: secrets that look like keys, tokens and private keys in the selected files are masked, and the dialog says how many were | nothing new — less | the send dialog | M |
| A5 | **"Local only"**: a setting that refuses every service whose host is not this machine | nothing | `ai.local_only` | S |

### B. Asking about what is on the screen

| # | Idea | What is sent | Where | Size |
| --- | --- | --- | --- | --- |
| B1 | **Explain this**: in the viewer and the editor, a key sends the marked block (or the screenful) with the file's name and type — a function in the disassembly, a stack trace in a log, a stanza of a config | the block, the name | viewer, editor | S |
| B2 | **Explain what failed**: after a command on the command line exits non-zero, a key sends the command and the end of its output | the command, the last 200 lines | the command line | S |
| B3 | **Ask the table**: in table mode, a question about the data, with the heading and a sample of rows (or the marked rows) | heading + rows, counted in the dialog | table mode | S |
| B4 | **Explain the difference**: on the internal diff (`Alt+F6`), a key asks what changed in words | the two sides of the diff | the diff view | S |
| B5 | **Describe a picture**: on an image, a vision-capable model says what is in it | the picture, scaled down | the image view | M |

### C. Producing something the program uses

These are the ones that make the model useful rather than chatty: the answer is a proposal that
lands in a place that already previews and confirms.

| # | Idea | What is sent | Where the proposal lands | Size |
| --- | --- | --- | --- | --- |
| C1 | **Words to a command**: `ai! find the photos over 5 MB from this year` puts a shell command on the command line, **not run**, with the shell and OS named in the prompt | the request, the shell, the OS | the command line | S |
| C2 | **Words to a search**: the same, filling the *Find file* dialog (masks, size, date, text) instead of a shell command | the request | `Alt+F7`, filled | M |
| C3 | **Words to a rename**: in multi-rename, describe the new names (`date taken, then the title`) and get the pattern, with the preview the dialog already shows | the request, a few of the selected names | `Ctrl+M`, filled | M |
| C4 | **A commit message from the staged diff**: in the git panel's `F7`, the prompt opens with a drafted message to edit | the staged diff, capped | the commit prompt | S |
| C5 | **Rewrite the block**: in the editor, a marked block is replaced by what the model returns — a fix, a translation, a reformat — shown first as a diff, applied as one undoable edit | the block, the instruction | the editor, via the diff | M |
| C6 | **Words to a filter**: in table mode, `Ctrl+F7` accepts a sentence and turns it into the filter | the request, the heading | table mode | S |

### D. Larger, later

| # | Idea | Why later | Size |
| --- | --- | --- | --- |
| D1 | **Semantic search** over a folder of text: an index of embeddings kept beside the tree cache, asked by meaning | an index to build, keep and invalidate; a local embedding model to choose | L |
| D2 | **A plan of file operations**: "sort these into folders by year" returns a list of moves the job queue shows and runs only on confirmation | the riskiest kind of proposal; needs a reviewable plan format first | L |
| D3 | **Tools for the model** (MCP or the services' own tool calls): the model may list a folder or read a file, each call shown and confirmed | every call is a prompt to the user; easy to make tiring, easy to make unsafe | L |
| D4 | **fcmd as an MCP server**, so an outside agent can drive panels | a second front door to the file system; the permission model has to exist first | L |

## What this study recommends

If one release is to take a step, the best value for its size is:

1. **A1 follow-ups** and **A3 cost** — they make the existing feature something one keeps
   using, and both are small.
2. **C1 words to a command** and **C4 commit message** — the two proposals that land in places
   that already confirm (the command line, the commit prompt), so they need no new safety at
   all, and both are things people already paste into a chat window by hand.
3. **B1 explain this** — the viewer and the editor are where an unfamiliar file is read, and a
   marked block is already the unit of "this part".

Then, in a release of its own, **C3 rename** and **C5 rewrite**, which need the most care in
showing the proposal, and **A4 redaction**, which should come before anything that sends whole
files by default.

**D** stays in the backlog until the rest has been used for a while: each of them is a new kind
of trust, not a new feature.

## Open questions for the author

* Is `ai!` (to a command) a good spelling beside `ai:` (to an answer), or should both be one
  prefix with the program deciding by what is asked?
* Should a follow-up send the earlier answers again (simple, costs tokens) or only a summary?
* Which local model should the documentation suggest for Ollama, so that C1 and C4 work offline
  out of the box?
* Does A4 (redaction) mask silently with a count, or stop and ask on each match?
