use std::{io, marker::PhantomData};

use thorn_core::{
    AppContext, Element, IntentMapper, KeyAction, KeyIntent, KeyMap, LayeredKeyMap, ThornApp,
};
use thorn_headless::TestRuntime;
use thorn_runtime::AppRuntime;
use thorn_terminal::TerminalRuntime;

pub fn app<State>(initial_state: State) -> AppBuilder<State> {
    AppBuilder { initial_state }
}

pub struct AppBuilder<State> {
    initial_state: State,
}

impl<State> AppBuilder<State> {
    pub fn update<Action, Update>(
        self,
        update: Update,
    ) -> AppBuilderWithUpdate<State, Action, Update>
    where
        Update: FnMut(&mut State, Action, &mut AppContext<Action>),
    {
        AppBuilderWithUpdate {
            state: self.initial_state,
            update,
            _action: PhantomData,
        }
    }
}

pub struct AppBuilderWithUpdate<State, Action, Update> {
    state: State,
    update: Update,
    _action: PhantomData<fn() -> Action>,
}

impl<State, Action, Update> AppBuilderWithUpdate<State, Action, Update>
where
    Update: FnMut(&mut State, Action, &mut AppContext<Action>),
{
    pub fn view<View>(self, view: View) -> ClosureApp<State, Action, Update, View>
    where
        View: Fn(&State) -> Element<Action>,
    {
        ClosureApp {
            state: self.state,
            update: self.update,
            view,
            runtime_config: FacadeRuntimeConfig::default(),
            _action: PhantomData,
        }
    }
}

pub struct ClosureApp<State, Action, Update, View> {
    state: State,
    update: Update,
    view: View,
    runtime_config: FacadeRuntimeConfig,
    _action: PhantomData<fn() -> Action>,
}

#[derive(Clone, Default)]
struct FacadeRuntimeConfig {
    keymap: Option<KeyMap>,
    layered_keymap: Option<LayeredKeyMap>,
    app_keymaps: Vec<KeyMap>,
    mode_keymaps: Vec<(&'static str, KeyMap)>,
}

impl<State, Action, Update, View> ClosureApp<State, Action, Update, View>
where
    Update: FnMut(&mut State, Action, &mut AppContext<Action>),
    View: Fn(&State) -> Element<Action>,
{
    pub fn keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime_config.keymap = Some(keymap);
        self.runtime_config.layered_keymap = None;
        self.runtime_config.app_keymaps.clear();
        self.runtime_config.mode_keymaps.clear();
        self
    }

    pub fn layered_keymap(mut self, keymap: LayeredKeyMap) -> Self {
        self.runtime_config.layered_keymap = Some(keymap);
        self.runtime_config.keymap = None;
        self.runtime_config.app_keymaps.clear();
        self.runtime_config.mode_keymaps.clear();
        self
    }

    pub fn app_keymap(mut self, keymap: KeyMap) -> Self {
        self.runtime_config.app_keymaps.push(keymap);
        self
    }

    pub fn mode_keymap(mut self, mode: &'static str, keymap: KeyMap) -> Self {
        self.runtime_config.mode_keymaps.push((mode, keymap));
        self
    }
}

impl<State, Action, Update, View> ClosureApp<State, Action, Update, View>
where
    Update: FnMut(&mut State, Action, &mut AppContext<Action>),
    View: Fn(&State) -> Element<Action>,
{
    pub fn run(self) -> io::Result<()> {
        let mut runtime = self.into_terminal_runtime_with_mapper(DefaultIntentMapper);
        runtime.run()
    }

    pub fn run_with_io(self, input: impl io::BufRead, output: impl io::Write) -> io::Result<()> {
        let mut runtime = self.into_terminal_runtime_with_mapper(DefaultIntentMapper);
        runtime.run_with_io(input, output)
    }

    pub fn into_runtime(self) -> AppRuntime<Self> {
        self.into_runtime_with_mapper(DefaultIntentMapper)
    }

    pub fn into_test_runtime(self) -> TestRuntime<Self> {
        self.into_test_runtime_with_mapper(DefaultIntentMapper)
    }

    pub fn into_runtime_with_mapper(
        self,
        mapper: impl IntentMapper<Action> + 'static,
    ) -> AppRuntime<Self> {
        let runtime_config = self.runtime_config.clone();
        let mut runtime = AppRuntime::new(self, mapper);
        if let Some(keymap) = runtime_config.keymap {
            runtime = runtime.keymap(keymap);
        }
        if let Some(layered_keymap) = runtime_config.layered_keymap {
            runtime = runtime.layered_keymap(layered_keymap);
        }
        for keymap in runtime_config.app_keymaps {
            runtime = runtime.app_keymap(keymap);
        }
        for (mode, keymap) in runtime_config.mode_keymaps {
            runtime = runtime.mode_keymap(mode, keymap);
        }
        runtime
    }

    pub fn into_test_runtime_with_mapper(
        self,
        mapper: impl IntentMapper<Action> + 'static,
    ) -> TestRuntime<Self> {
        let runtime_config = self.runtime_config.clone();
        let mut runtime = TestRuntime::new(self, mapper);
        if let Some(keymap) = runtime_config.keymap {
            runtime = runtime.keymap(keymap);
        }
        if let Some(layered_keymap) = runtime_config.layered_keymap {
            runtime = runtime.layered_keymap(layered_keymap);
        }
        for keymap in runtime_config.app_keymaps {
            runtime = runtime.app_keymap(keymap);
        }
        for (mode, keymap) in runtime_config.mode_keymaps {
            runtime = runtime.mode_keymap(mode, keymap);
        }
        runtime
    }

    fn into_terminal_runtime_with_mapper(
        self,
        mapper: impl IntentMapper<Action> + 'static,
    ) -> TerminalRuntime<Self> {
        let runtime_config = self.runtime_config.clone();
        let mut runtime = TerminalRuntime::new(self, mapper);
        if let Some(keymap) = runtime_config.keymap {
            runtime = runtime.keymap(keymap);
        }
        if let Some(layered_keymap) = runtime_config.layered_keymap {
            runtime = runtime.layered_keymap(layered_keymap);
        }
        for keymap in runtime_config.app_keymaps {
            runtime = runtime.app_keymap(keymap);
        }
        for (mode, keymap) in runtime_config.mode_keymaps {
            runtime = runtime.mode_keymap(mode, keymap);
        }
        runtime
    }
}

impl<State, Action, Update, View> ThornApp for ClosureApp<State, Action, Update, View>
where
    Update: FnMut(&mut State, Action, &mut AppContext<Action>),
    View: Fn(&State) -> Element<Action>,
{
    type Action = Action;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>) {
        (self.update)(&mut self.state, action, ctx);
    }

    fn view(&self) -> Element<Self::Action> {
        (self.view)(&self.state)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultIntentMapper;

impl<Action> IntentMapper<Action> for DefaultIntentMapper {
    fn map_intent(&self, intent: KeyIntent) -> Option<KeyAction<Action>> {
        match intent {
            KeyIntent::RequestQuit => Some(KeyAction::RuntimeQuit),
            _ => None,
        }
    }
}
