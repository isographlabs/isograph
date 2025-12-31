use crossbeam::channel::Sender;
use isograph_compiler::CompilerState;
use isograph_schema::CompilationProfile;

pub struct LspState<'a, TCompilationProfile: CompilationProfile> {
    pub compiler_state: CompilerState<TCompilationProfile>,
    pub sender: &'a Sender<lsp_server::Message>,
}

impl<'a, TCompilationProfile: CompilationProfile> LspState<'a, TCompilationProfile> {
    pub fn new(
        compiler_state: CompilerState<TCompilationProfile>,
        sender: &'a Sender<lsp_server::Message>,
    ) -> Self {
        LspState {
            compiler_state,
            sender,
        }
    }
}
