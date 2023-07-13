//! Module for patching the DOM with the [Patch] type that express the difference between the last
//! evaluation of the virtual dom and the current one.

use roko_html::{Attribute, Html};

use dom::{HtmlCollection, HtmlElement};
use futures::SinkExt;

use wasm_bindgen::JsCast;
use web_sys as dom;

use crate::render::{Context, Render};

/// Patch for attributes
pub enum AttrPatch<Msg> {
    Add(Attribute<Msg>),
    Remove(Attribute<Msg>),
}

/// The patch type that express the difference between the last evaluation of the virtual dom and
/// the current one.
pub enum Patch<Msg> {
    Add(Html<Msg>),
    Replace(Html<Msg>),
    Update(Vec<Patch<Msg>>, Vec<AttrPatch<Msg>>),
    Remove,
    Nothing,
}

/// Applies a sequence of pathes for children.
fn apply_children<Msg: 'static + Send + Sync>(
    parent: dom::Element,
    children: HtmlCollection,
    patches: Vec<Patch<Msg>>,
    context: &mut Context<'_, Msg>,
) {
    for (i, patch) in patches.into_iter().enumerate() {
        if let Some(child) = children.get_with_index(i as u32) {
            patch.apply(child, context);
        } else {
            patch.apply(parent.clone(), context);
        }
    }
}

/// Applies a sequence of patches for a sequence of attributes.
fn apply_attributes<Msg: 'static + Send + Sync>(
    el: dom::Element,
    patches: Vec<AttrPatch<Msg>>,
    context: &mut Context<'_, Msg>,
) {
    for patch in patches {
        match patch {
            AttrPatch::Add(add) => {
                add.render(el.clone(), context);
            }
            AttrPatch::Remove(rem) => match rem {
                Attribute::OnClick(_) => el.dyn_ref::<HtmlElement>().unwrap().set_onclick(None),
                Attribute::Custom(n, _) => el.set_attribute(&n, "").unwrap(),
                Attribute::OnMount(_) => (),
                Attribute::OnUnmount(ev) => {
                    let ev = ev.clone();
                    let context = context.channel.clone();

                    let ev_future = async move { context.clone().send(ev).await };

                    futures::executor::block_on(ev_future).unwrap();
                }
            },
        }
    }
}

impl<'a, Msg: 'static + Send + Sync> Patch<Msg> {
    /// This function applies a patch to the real dom.
    pub fn apply(self, el: dom::Element, context: &mut Context<'a, Msg>) {
        match self {
            Patch::Add(add) => {
                if let Some(el) = add.render(el, context) {
                    el.append_child(&el).unwrap();
                }
            }
            Patch::Replace(replace) => {
                if let Some(el) = replace.render(el, context) {
                    el.replace_with_with_node_1(&el).unwrap();
                }
            }
            Patch::Update(children, attr) => {
                apply_children(el.clone(), el.children(), children, context);
                apply_attributes(el, attr, context);
            }
            Patch::Remove => el.remove(),
            Patch::Nothing => (),
        }
    }
}
