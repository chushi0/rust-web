use std::rc::Rc;

use yew::prelude::*;
use yew_router::prelude::*;

const NAV_TITLE: &str = "CSZT";

#[derive(PartialEq, Properties)]
pub struct NavBarProps {
    pub active: &'static str,
}

#[function_component]
pub fn NavBar(props: &NavBarProps) -> Html {
    let nav_items = vec![
        Item::Item {
            id: "home",
            title: "动态",
            href: crate::Route::Home,
        },
        Item::Menu {
            title: "实验",
            children: Rc::new(vec![]),
        },
    ];

    html! {
        <nav class="navbar navbar-expand-lg navbar-dark bg-primary" style="margin-bottom: 1em;">
            <div class="container-fluid">
                <Link<crate::Route> classes={classes!("navbar-brand")} to={crate::Route::Home}>{NAV_TITLE}</Link<crate::Route>>
                <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#navbarNav" aria-controls="navbarNav" aria-expanded="false" aria-label="菜单">
                    <span class="navbar-toggler-icon"></span>
                </button>
                <div class="collapse navbar-collapse" id="navbarNav">
                    <ul class="navbar-nav">
                        {nav_items.iter().map(|item| html! {
                            <NavItem item={item.clone()} active={props.active}/>
                        }).collect::<Html>()}
                    </ul>
                </div>
            </div>
        </nav>
    }
}

#[derive(PartialEq, Clone)]
enum Item {
    Item {
        id: &'static str,
        title: &'static str,
        href: crate::Route,
    },
    Menu {
        title: &'static str,
        children: Rc<Vec<Item>>,
    },
    // Divider,
}
impl Item {
    fn is_active(&self, active_id: &'static str) -> bool {
        match self {
            Item::Item {
                id,
                title: _,
                href: _,
            } => *id == active_id,
            Item::Menu { title: _, children } => {
                for item in children.iter() {
                    if item.is_active(active_id) {
                        return true;
                    }
                }
                false
            } // _ => false,
        }
    }
}

#[derive(PartialEq, Properties)]
struct NavItemProps {
    item: Item,
    active: &'static str,
    #[prop_or_default]
    dropdown_item: bool,
}

#[function_component]
fn NavItem(props: &NavItemProps) -> Html {
    let li_class = if props.dropdown_item {
        classes!()
    } else {
        classes!("nav-item")
    };
    match &props.item {
        Item::Item { id, title, href } => {
            let mut link_class = classes!();
            if props.dropdown_item {
                link_class.push("dropdown-item")
            } else {
                link_class.push("nav-link")
            }
            if *id == props.active {
                link_class.push("active")
            }
            html! {
                <li class={li_class}>
                    <Link<crate::Route> classes={link_class} to={*href}>{title}</Link<crate::Route>>
                </li>
            }
        }
        Item::Menu { title, children } => {
            let mut dropdown_class = classes!("dropdown-toggle");
            if props.dropdown_item {
                dropdown_class.push("dropdown-item")
            } else {
                dropdown_class.push("nav-link")
            }
            if props.item.is_active(props.active) {
                dropdown_class.push("active");
            }
            html! {
                <li class={classes!(li_class, "dropdown")}>
                    <a class={dropdown_class} href="#" role="button" data-bs-toggle="dropdown">
                        {title}
                    </a>
                    <ul class="dropdown-menu">
                        {children.iter().map(|item| html! {
                            <NavItem item={item.clone()} active={props.active} dropdown_item=true/>
                        }).collect::<Html>()}
                    </ul>
                </li>
            }
        }
        // Item::Divider => html! {
        //     <li><hr class="dropdown-divider" /></li>
        // },
    }
}
