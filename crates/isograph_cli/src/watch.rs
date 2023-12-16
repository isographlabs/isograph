use colored::Colorize;
use futures::{
    channel::mpsc::{channel, Receiver},
    SinkExt, StreamExt,
};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;

use crate::{
    batch_compile::handle_compile_command, config::CompilerConfig, opt::BatchCompileCliOptions,
};

pub(crate) fn handle_watch_command(opt: BatchCompileCliOptions) {
    let config = CompilerConfig::create(opt.config.as_ref());

    match handle_compile_command(&opt) {
        Ok(_) => eprintln!("{}", "Successfully compiled.\n".bright_green()),
        Err(err) => {
            eprintln!("{}\n{}", "Error when compiling.\n".bright_red(), err);
        }
    };

    futures::executor::block_on(async {
        if let Err(e) = async_watch(config.project_root, &opt).await {
            println!("error: {:?}", e);
        }
    });
}

fn async_watcher() -> notify::Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (mut tx, rx) = channel(1);

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = RecommendedWatcher::new(
        move |res| {
            futures::executor::block_on(async {
                tx.send(res).await.unwrap();
            })
        },
        Config::default(),
    )?;

    Ok((watcher, rx))
}

async fn async_watch<P: AsRef<Path>>(path: P, opt: &BatchCompileCliOptions) -> notify::Result<()> {
    let (mut watcher, mut rx) = async_watcher()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

    while let Some(res) = rx.next().await {
        match res {
            Ok(_) => {
                println!("{}\n", "Detected file event, recompiling.".bright_green());

                match handle_compile_command(&opt) {
                    Ok(_) => eprintln!("{}", "Successfully compiled.\n".bright_green()),
                    Err(err) => {
                        eprintln!("{}\n{}", "Error when compiling.\n".bright_red(), err);
                    }
                };
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
