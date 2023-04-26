use crate::settings::*;
use include_gif::include_gif;
use ledger_prompts_ui::*;
use nanos_ui::bagls::*;
use nanos_ui::bitmaps::Glyph;

pub const APP_ICON_GLYPH: Glyph = Glyph::from_include(include_gif!("pocket-small.gif"));

pub const APP_ICON: Icon = Icon::from(&APP_ICON_GLYPH)
    .set_x(MENU_ICON_X)
    .set_y(MENU_ICON_Y);

pub struct IdleMenuWithSettings {
    pub idle_menu: IdleMenu,
    pub settings: Settings,
}

pub enum IdleMenu {
    AppMain,
    ShowVersion,
    Exit,
}

pub enum BusyMenu {
    Working,
    Cancel,
}

pub struct DoExitApp;

impl Menu for IdleMenuWithSettings {
    type BothResult = DoExitApp;
    fn move_left(&mut self) {
        use crate::menu::IdleMenu::*;
        match self.idle_menu {
            AppMain => self.idle_menu = Exit,
            ShowVersion => self.idle_menu = AppMain,
            Exit => self.idle_menu = ShowVersion,
        };
    }
    fn move_right(&mut self) {
        use crate::menu::IdleMenu::*;
        match self.idle_menu {
            AppMain => self.idle_menu = ShowVersion,
            ShowVersion => self.idle_menu = Exit,
            Exit => self.idle_menu = AppMain,
        };
    }
    #[inline(never)]
    fn handle_both(&mut self) -> Option<Self::BothResult> {
        use crate::menu::IdleMenu::*;
        match self.idle_menu {
            AppMain => None,
            ShowVersion => None,
            Exit => Some(DoExitApp),
        }
    }
    #[inline(never)]
    fn label<'a>(&self) -> (MenuLabelTop<'a>, MenuLabelBottom<'a>) {
        use crate::menu::IdleMenu::*;
        match self.idle_menu {
            AppMain => (
                MenuLabelTop::Icon(&APP_ICON),
                MenuLabelBottom {
                    text: "Pocket",
                    bold: true,
                },
            ),
            ShowVersion => (
                MenuLabelTop::Text("Version"),
                MenuLabelBottom {
                    text: env!("CARGO_PKG_VERSION"),
                    bold: false,
                },
            ),
            Exit => (
                MenuLabelTop::Icon(&DASHBOARD_ICON),
                MenuLabelBottom {
                    text: "Quit",
                    bold: true,
                },
            ),
        }
    }
}

pub struct DoCancel;

impl Menu for BusyMenu {
    type BothResult = DoCancel;
    fn move_left(&mut self) {
        *self = BusyMenu::Working;
    }
    fn move_right(&mut self) {
        *self = BusyMenu::Cancel;
    }
    #[inline(never)]
    fn handle_both(&mut self) -> Option<Self::BothResult> {
        use crate::menu::BusyMenu::*;
        match self {
            Working => None,
            Cancel => Some(DoCancel),
        }
    }
    #[inline(never)]
    fn label<'a>(&self) -> (MenuLabelTop<'a>, MenuLabelBottom<'a>) {
        use crate::menu::BusyMenu::*;
        match self {
            Working => (
                MenuLabelTop::Text("Working..."),
                MenuLabelBottom {
                    text: "",
                    bold: false,
                },
            ),
            Cancel => (
                MenuLabelTop::Text("Cancel"),
                MenuLabelBottom {
                    text: "",
                    bold: false,
                },
            ),
        }
    }
}
