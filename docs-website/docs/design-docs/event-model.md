# Event model

The isograph process is the same shape as figaro: sources send events into one channel; an event loop owns the state and dispatches each event into inert effects; an effect loop performs them. Dispatch does no IO. Sources do not touch the state.

A binary is compiled with one `HostLanguage`, one `NetworkProtocol`, and one `RuntimeFramework`. Those are static for the process. The config file's shape depends on them.

Figaro is one process on the machine. Isograph is one process per config file. `freddie_cli` is the same in both: figaro's `Instance` is `global`; isograph's is `named` for that config.

The daemon is that process. Watch mode and the LSP are not separate programs. They share one pico database.

```text
file watcher, LSP, SIGTERM, async work
        |
        v
   IsographEvent  ----->  dispatch(state, event)  ----->  IsographEffect
                                |                              |
                         pico database                    effect loop
                         (the state)                    (IO happens here)
```

## State

The state is a pico database. Disk files and open editor buffers are source nodes. Compilation is derived. Watch mode and the LSP read the same nodes.

The model distinguishes the file on disk from the file open in the editor. Artifact generation and watch mode read the disk. The LSP does not generate artifacts. Whether the LSP should generate artifacts is open.

## Events

An event is something that happened, already carrying what the source knows.

- A file on disk changed (created, written, removed). The watcher names the path and whether it is present.
- An open editor buffer changed. The LSP names the path and the buffer text.
- Async work finished. Compilation, a schema fetch, anything dispatch asked the effect loop to do off-thread. The result rides the event back into dispatch.
- Quit. `isograph stop` and SIGTERM are this.

## Effects

An effect is inert data. The effect loop is the only place the outside world is written: artifacts, diagnostics, a compile kicked off the event thread, process exit.

## Watch and batch

After the daemon exists, the first milestone is watch mode: listen for disk changes, compile, print errors.

Batch mode is watch mode from a fresh start. Boot scans the project as a burst of "file changed" events, then the same loop keeps running. There is no second pipeline.
