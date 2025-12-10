# async-job-orchestrator

A TOY asynchronous job orchestration framework in Rust, developed as a project for me to learn Rust! 

- It is currently WIP but has been extremely educational to develop
- Human-coded, nothing authored by AI (except the PRD I had it generate for me based on what I wanted the project to be, not sure I've stayed true to it, lol)

## What is It?

- An axum-based server that accepts two types of jobs: 'echo' and 'sleep' (see if you can guess what these jobs do)
- A fixed-length server-side job pool that manages queuing and running of jobs
- Job submission handling loop using tokio-based aync/await concurrency
- Parallel job execution in OS threads via spawn_blocking   
- Per-job fixed-size logging buffer

## What am I trying to Learn about Rust? 
- Write a non-trivial program to learn significant things
- General Rust syntax and semantics
- Concurrency and parallelism
- Memory use in a borrow-checked world: Box, Arc, Arc<Mutex<>> and friends
- Anything else that comes up :-) 

