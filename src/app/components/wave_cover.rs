use image::DynamicImage;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    symbols::border,
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
};
use ratatui_image::{StatefulImage, protocol::StatefulProtocol};
use std::sync::Arc;
use unicode_width::UnicodeWidthStr;

use crate::{app::theme::theme, cache::image::ImageCache, framework::reactive::Signal};

const LYRIC_GAP: u16 = 1;

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

pub struct WaveCover {
    cover_url: Signal<Option<String>>,
    protocol: Option<StatefulProtocol>,
    last_art: Option<Arc<DynamicImage>>,
}

impl WaveCover {
    pub fn new(cover_url: Signal<Option<String>>) -> Self {
        Self {
            cover_url,
            protocol: None,
            last_art: None,
        }
    }

    pub fn prepare(&mut self) {
        let cache = ImageCache::global();
        let current_art = self.cover_url.get().and_then(|url| cache.get_or_fetch(&url));

        let art_changed = match (&self.last_art, &current_art) {
            (Some(old), Some(new)) => !Arc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        };
        if art_changed {
            self.protocol = None;
            if let (Some(picker), Some(art)) = (ImageCache::global_picker(), &current_art) {
                self.protocol = Some(picker.new_resize_protocol((**art).clone()));
            }
            self.last_art = current_art;
        }
    }

    pub fn image_rect(&self, area: Rect, has_lyric: bool) -> Option<Rect> {
        if area.width == 0 || area.height == 0 {
            return None;
        }
        let art = self.last_art.as_ref()?;
        let picker = ImageCache::global_picker()?;
        let font = picker.font_size();
        let fw = (font.width as u32).max(1);
        let fh = (font.height as u32).max(1);
        let lcm = fw / gcd(fw, fh) * fh;

        let max_h = (((area.height as f32) * 0.45) as u16).clamp(6, 28) as u32;
        let art_side = art.width().min(art.height()).max(1);
        let budget = (max_h * fh).min((area.width as u32) * fw).min(art_side);
        let side = ((budget / lcm) * lcm).max(lcm);

        let img_w = (side / fw) as u16;
        let img_h = (side / fh) as u16;
        if img_w == 0 || img_h == 0 || img_w > area.width || img_h > area.height {
            return None;
        }

        let lyric_h: u16 = if has_lyric { LYRIC_GAP + 3 } else { 0 };
        let group_h = img_h + lyric_h;
        let top = area.y + (area.height.saturating_sub(group_h)) / 2;
        let img_x = area.x + (area.width.saturating_sub(img_w)) / 2;
        Some(Rect {
            x: img_x,
            y: top,
            width: img_w,
            height: img_h,
        })
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, cover: Option<Rect>, lyric: Option<&str>) {
        let Some(img_rect) = cover else {
            return;
        };

        if let Some(proto) = &mut self.protocol {
            frame.render_stateful_widget(StatefulImage::new(), img_rect, proto);
        }

        if let Some(text) = lyric {
            let bg = theme().bg.base;
            let text_w = (UnicodeWidthStr::width(text) as u16).min(area.width.saturating_sub(4));
            let box_w = (text_w + 4).min(area.width);
            let box_x = area.x + (area.width.saturating_sub(box_w)) / 2;
            let box_rect = Rect {
                x: box_x,
                y: img_rect.bottom() + LYRIC_GAP,
                width: box_w,
                height: 3,
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(theme().focused_border)
                .style(Style::default().bg(bg));
            let inner = block.inner(box_rect);
            frame.render_widget(Clear, box_rect);
            frame.render_widget(block, box_rect);

            frame.render_widget(
                Paragraph::new(Span::styled(
                    text.to_string(),
                    Style::default()
                        .fg(theme().accent.primary)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ))
                .style(Style::default().bg(bg))
                .alignment(Alignment::Center),
                inner,
            );
        }
    }
}
