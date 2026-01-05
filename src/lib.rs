use std::io::{self, Write};

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
}

impl Default for Cell {
    fn default() -> Self {
        Cell { ch: ' ' }
    }
}
pub trait DrawTarget {
    fn clear(&mut self);
    fn put_char(&mut self, x: usize, y: usize, ch: char);
    fn write_str(&mut self, x: usize, y: usize, text: &str);
    fn write_i64_right(&mut self, x: usize, y: usize, value: i64, width: usize);
    fn write_f64_right(&mut self, x: usize, y: usize, value: f64, width: usize, precision: usize);
    fn flush(&self);
    fn draw_hline(&mut self, x: usize, y: usize, w: usize, ch: char);
    fn draw_vline(&mut self, x: usize, y: usize, h: usize, ch: char);
    fn draw_frame(&mut self, x: usize, y: usize, w: usize, h: usize);
}
pub struct ScreenBuffer {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}
impl ScreenBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}
impl DrawTarget for ScreenBuffer {
    fn clear(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }
    fn put_char(&mut self, x: usize, y: usize, ch: char) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.index(x, y);
        self.cells[idx].ch = ch;
    }
    fn write_str(&mut self, x: usize, y: usize, text: &str) {
        if y >= self.height {
            return;
        }
        for (i, ch) in text.chars().enumerate() {
            let px = x + i;
            if px >= self.width {
                return;
            }
            self.put_char(px, y, ch);
        }
    }
    fn write_i64_right(&mut self, x: usize, y: usize, mut value: i64, width: usize) {
        if y >= self.height {
            return;
        }

        // needed if we want to support popups like overwriting the layer beneath
        for i in 0..width {
            self.put_char(x + i, y, ' ');
        }

        if value == 0 {
            if width > 0 {
                self.put_char(x + width - 1, y, '0');
            }
            return;
        }
        let negative = value < 0;
        if negative {
            value = -value;
        }

        let mut pos = x + width;

        while value > 0 && pos > x {
            pos -= 1;
            let digit = (value % 10) as u8;
            self.put_char(pos, y, char::from(b'0' + digit));
            value /= 10;
        }

        if negative && pos > x {
            self.put_char(pos - 1, y, '-');
        }
    }
    fn write_f64_right(&mut self, x: usize, y: usize, value: f64, width: usize, precision: usize) {
        if y >= self.height {
            return;
        }

        let scale = 10_i64.pow(precision as u32);
        let scaled = (value * scale as f64).round() as i64;

        let int_part = scaled / scale;
        let mut fract_part = (scaled % scale).abs();

        for i in 0..width {
            self.put_char(x + i, y, ' ');
        }

        let mut pos = x + width;

        for _ in 0..precision {
            if pos <= x {
                return;
            }
            pos -= 1;
            let d = (fract_part % 10) as u8;
            self.put_char(pos, y, char::from(b'0' + d));
            fract_part /= 10;
        }

        if precision > 0 && pos > x {
            pos -= 1;
            self.put_char(pos, y, '.');
        }
        let mut v = int_part.abs();
        if v == 0 && pos > x {
            pos -= 1;
            self.put_char(pos, y, '0');
        } else {
            while v > 0 && pos > x {
                pos -= 1;
                let d = (v % 10) as u8;
                self.put_char(pos, y, char::from(b'0' + d));
                v /= 10;
            }
        }
        if int_part < 0 && pos > x {
            self.put_char(pos - 1, y, '-');
        }
    }
    fn flush(&self) {
        let mut out = String::with_capacity(self.width * self.height + self.height);

        out.push_str("\x1B[2J\x1B[H");

        for y in 0..self.height {
            for x in 0..self.width {
                out.push(self.cells[self.index(x, y)].ch);
            }
            out.push('\n');
        }
        print!("{}", out);
        io::stdout().flush().unwrap();
    }
    fn draw_hline(&mut self, x: usize, y: usize, w: usize, ch: char) {
        for i in 0..w {
            if x + 1 >= self.width {
                return;
            }
            self.put_char(x + i, y, ch);
        }
    }
    fn draw_vline(&mut self, x: usize, y: usize, h: usize, ch: char) {
        for i in 0..h {
            if y + 1 >= self.width {
                return;
            }
            self.put_char(x, y + i, ch);
        }
    }
    fn draw_frame(&mut self, x: usize, y: usize, w: usize, h: usize) {
        self.put_char(x, y, '┌');
        self.put_char(x + w - 1, y, '┐');
        self.put_char(x, y + h - 1, '└');
        self.put_char(x + w - 1, y + h - 1, '┘');

        self.draw_hline(x + 1, y, w - 2, '-');
        self.draw_hline(x + 1, y + h - 1, w - 2, '-');
        self.draw_vline(x, y + 1, h - 2, '|');
        self.draw_vline(x + w - 1, y + 1, h - 2, '|');
    }
}
pub enum BorderKind {
    Full,
    No,
}
enum LayoutKind {
    Vertical,
    Horizontal,
}
pub struct UiGrid<'a, 'b, T>
where
    T: DrawTarget,
{
    parent: &'b mut Ui<'a, T>,
    start_x: usize,
    start_y: usize,
    cols: usize,
    spacing: usize,
    spacing_inner: usize,
    cell_idx: usize,
    cursor_x: usize,
    cursor_y: usize,
    max_col_width: Vec<usize>,
    max_row_height: Vec<usize>,
    draw: bool,
}
impl<'a, 'b, T> UiGrid<'a, 'b, T>
where
    T: DrawTarget,
{
    pub fn cell(&mut self, f: impl Fn(&mut Ui<T>)) {
        let col = self.cell_idx % self.cols;
        let row = self.cell_idx / self.cols;

        if self.max_col_width.len() < self.cols {
            self.max_col_width.resize(self.cols, 0);
        }

        if self.max_row_height.len() <= row {
            self.max_row_height.resize(row + 1, 0);
        }

        let start_x = self.start_x
            + self.max_col_width[..col].iter().sum::<usize>()
            + col * self.spacing_inner;
        let start_y = self.start_y
            + self.max_row_height[..row].iter().sum::<usize>()
            + row * self.spacing_inner;

        let mut cell_ui = Ui {
            buf: self.parent.buf,
            cursor_x: start_x,
            cursor_y: start_y,
            max_x: start_x,
            max_y: start_y,
            // TODO: adjust to col width, row height
            available_x: Some(self.max_col_width[col]),
            available_y: Some(self.max_row_height[row]),
            used_x: 0,
            used_y: 0,
            layout: LayoutKind::Horizontal,
            spacing: self.spacing,
            draw: self.draw,
        };
        f(&mut cell_ui);
        let used_w = cell_ui.max_x - start_x;
        self.max_col_width[col] = self.max_col_width[col].max(used_w);

        let used_h = cell_ui.max_y - start_y;
        self.max_row_height[row] = self.max_row_height[row].max(used_h);

        self.cell_idx += 1;
    }
}
pub enum StretchHint {
    Full,
    Compact,
}
pub enum Align {
    Left,
    Right,
}
// TODO: Add available_w, available_h to support strech in child.
pub struct Ui<'a, T: DrawTarget> {
    buf: &'a mut T,
    cursor_x: usize,
    cursor_y: usize,
    max_x: usize,
    max_y: usize,
    available_x: Option<usize>,
    available_y: Option<usize>,
    used_x: usize,
    used_y: usize,
    layout: LayoutKind,
    spacing: usize,
    draw: bool,
}
impl<'a, T> Ui<'a, T>
where
    T: DrawTarget,
{
    pub fn new(buf: &'a mut T, x: usize, y: usize) -> Self {
        Ui {
            buf,
            cursor_x: x,
            cursor_y: y,
            max_x: x,
            max_y: y,
            available_x: None,
            available_y: None,
            used_x: 0,
            used_y: 0,
            layout: LayoutKind::Vertical,
            spacing: 0,
            draw: true,
        }
    }
    pub fn flush(&mut self) {
        self.buf.flush();
    }
    pub fn clear(&mut self) {
        self.buf.clear();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.max_x = 0;
        self.max_y = 0;
        self.available_x = None;
        self.available_y = None;
        self.used_x = 0;
        self.used_y = 0;
        self.layout = LayoutKind::Vertical;
        self.spacing = 0;
    }
    fn advance(&mut self, w: usize, h: usize) {
        self.max_x = self.max_x.max(self.cursor_x + w);
        self.max_y = self.max_y.max(self.cursor_y + h);

        match self.layout {
            LayoutKind::Vertical => {
                self.used_x = self.used_x.max(w);
                if let Some(avail_y) = self.available_y {
                    self.available_y = avail_y.checked_sub(h);
                }
                self.cursor_y += h + self.spacing;
            }
            LayoutKind::Horizontal => {
                self.used_y = self.used_y.max(h);
                if let Some(avail_x) = self.available_x {
                    self.available_x = avail_x.checked_sub(w);
                }
                self.cursor_x += w + self.spacing;
            }
        }
    }
    fn child(&mut self, layout: LayoutKind, spacing: usize, f: impl FnOnce(&mut Ui<T>)) {
        let start_x = self.cursor_x;
        let start_y = self.cursor_y;

        let mut child = Ui {
            buf: self.buf,
            cursor_x: start_x,
            cursor_y: start_y,
            max_x: start_x,
            max_y: start_y,
            available_x: self.available_x,
            available_y: self.available_y,
            used_x: 0,
            used_y: 0,
            layout,
            spacing,
            draw: self.draw,
        };
        f(&mut child);

        let used_w = match child.layout {
            LayoutKind::Vertical => child.used_x,
            LayoutKind::Horizontal => child.max_x - start_x,
        };
        let used_h = match child.layout {
            LayoutKind::Vertical => child.max_y - start_y,
            LayoutKind::Horizontal => child.used_y,
        };
        self.advance(used_w, used_h);
    }
    fn draw_frame(&mut self, x: usize, y: usize, w: usize, h: usize) {
        if !self.draw {
            return;
        }
        let buf = &mut self.buf;
        for dx in 0..w {
            buf.put_char(x + dx, y, '-');
            buf.put_char(x + dx, y + h - 1, '-');
        }
        for dy in 0..h {
            buf.put_char(x, y + dy, '|');
            buf.put_char(x + w - 1, y + dy, '|');
        }

        buf.put_char(x, y, '+');
        buf.put_char(x + w - 1, y, '+');
        buf.put_char(x, y + h - 1, '+');
        buf.put_char(x + w - 1, y + h - 1, '+');
    }
    pub fn space(&mut self, amount: usize) {
        match self.layout {
            LayoutKind::Vertical => self.advance(0, amount),
            LayoutKind::Horizontal => self.advance(amount, 0),
        }
    }
    pub fn vertical(&mut self, f: impl FnOnce(&mut Ui<T>)) {
        self.child(LayoutKind::Vertical, self.spacing, f);
    }
    pub fn horizontal(&mut self, f: impl FnOnce(&mut Ui<T>)) {
        self.child(LayoutKind::Horizontal, self.spacing, f);
    }
    pub fn grid(&mut self, cols: usize, spacing: usize, f: impl Fn(&mut UiGrid<T>)) {
        let start_x = self.cursor_x;
        let start_y = self.cursor_y;

        let mut tmp_grid = UiGrid {
            spacing: self.spacing,
            parent: self,
            start_x,
            start_y,
            cols,
            spacing_inner: spacing,
            cell_idx: 0,
            cursor_x: 0,
            cursor_y: 0,
            max_col_width: vec![0; cols],
            max_row_height: vec![0],
            draw: false,
        };
        f(&mut tmp_grid);
        let measured_max_col_width = tmp_grid.max_col_width;
        let measured_max_row_height = tmp_grid.max_row_height;

        let mut grid = UiGrid {
            spacing: self.spacing,
            parent: self,
            start_x,
            start_y,
            cols,
            spacing_inner: spacing,
            cell_idx: 0,
            cursor_x: 0,
            cursor_y: 0,
            max_col_width: measured_max_col_width,
            max_row_height: measured_max_row_height,
            draw: true,
        };
        f(&mut grid);

        let used_w = grid.max_col_width.iter().sum::<usize>()
            + grid.spacing_inner * (cols.saturating_sub(1));
        let used_h = grid.max_row_height.iter().sum::<usize>()
            + grid.spacing_inner * grid.max_row_height.len().saturating_sub(1);
        self.advance(used_w, used_h);
    }
    pub fn frame(
        &mut self,
        padding: usize,
        border: BorderKind,
        stretch: StretchHint,
        f: impl FnOnce(&mut Ui<T>),
    ) {
        let start_x = self.cursor_x;
        let start_y = self.cursor_y;

        let avail_x = if let Some(x) = self.available_x {
            if x.saturating_sub(2 * padding) > 0 {
                Some(x - 2 * padding)
            } else {
                None
            }
        } else {
            None
        };
        let avail_y = if let Some(y) = self.available_y {
            if y.saturating_sub(2 * padding) > 0 {
                Some(y - 2 * padding)
            } else {
                None
            }
        } else {
            None
        };
        let mut child = Ui {
            buf: self.buf,
            cursor_x: start_x + padding,
            cursor_y: start_y + padding,
            max_x: start_x + padding,
            max_y: start_y + padding,
            // TODO: should depend on whether frame is compact or full not yet implemented
            available_x: avail_x,
            available_y: avail_y,
            used_x: 0,
            used_y: 0,
            layout: LayoutKind::Vertical,
            spacing: self.spacing,
            draw: self.draw,
        };

        f(&mut child);

        let mut used_w = child.max_x - start_x + padding;
        let mut used_h = child.max_y - start_y + padding;

        match stretch {
            StretchHint::Full => {
                used_w = used_w.max(self.available_x.unwrap_or(0));

                used_h = used_h.max(self.available_y.unwrap_or(0))
            }
            StretchHint::Compact => {}
        }

        match border {
            BorderKind::Full => self.draw_frame(start_x, start_y, used_w, used_h),
            BorderKind::No => {}
        }
        self.advance(used_w, used_h);
    }
    pub fn label_align(
        &mut self,
        text: &str,
        width: Option<usize>,
        align_inner: Align,
        align_outer: Align,
    ) {
        let len = text.len();
        let w = width.unwrap_or(len);
        let visible_len = len.min(w);

        let slice = if len > w { &text[..w] } else { text };
        // outer
        let start_x = if let Some(avail_x) = self.available_x {
            match align_outer {
                Align::Left => self.cursor_x,
                Align::Right => self.cursor_x + avail_x.saturating_sub(w),
            }
        } else {
            // no right border known, that we can align to
            self.cursor_x
        };
        // inner
        let start_x = match align_inner {
            Align::Left => start_x,
            Align::Right => start_x + w.saturating_sub(visible_len),
        };
        if self.draw {
            for i in 0..w {
                self.buf.put_char(self.cursor_x + i, self.cursor_y, ' ');
            }
            self.buf.write_str(start_x, self.cursor_y, slice);
        }
        self.used_x = self.used_x.max(w);
        self.advance(w, 1);
    }
    pub fn label(&mut self, text: &str, width: Option<usize>, align: Align) {
        let len = text.len();
        let w = width.unwrap_or(len);
        let visible_len = len.min(w);

        let slice = if len > w { &text[..w] } else { text };
        let start_x = match align {
            Align::Left => self.cursor_x,
            Align::Right => self.cursor_x + w - visible_len,
        };
        if self.draw {
            for i in 0..w {
                self.buf.put_char(self.cursor_x + i, self.cursor_y, ' ');
            }
            self.buf.write_str(start_x, self.cursor_y, slice);
        }
        self.advance(w, 1);
    }
    pub fn number_i64(&mut self, value: i64, width: usize) {
        if self.draw {
            self.buf
                .write_i64_right(self.cursor_x, self.cursor_y, value, width);
        }
        self.advance(width, 1);
    }
    // TODO: BUG: continue to writes to the left
    pub fn number_f64(&mut self, value: f64, precision: usize, width: usize) {
        if self.draw {
            self.buf
                .write_f64_right(self.cursor_x, self.cursor_y, value, width, precision);
        }
        self.advance(width, 1);
    }
}
trait Layout {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn init_position_from_other(&mut self, x: usize, y: usize);
    fn init_position_other_layout<L: Layout>(&self, layout: &mut L);
    fn update_self_size_from_other<L: Layout>(&mut self, layout: &L);
}
struct VLayout {
    x: usize,
    y: usize,
    gap: usize,
    current_y: usize,
    width: usize,
}
impl VLayout {
    fn new(x: usize, y: usize, gap: usize) -> Self {
        Self {
            x,
            y,
            gap,
            current_y: y,
            width: 0,
        }
    }
    fn write_str(&mut self, buf: &mut ScreenBuffer, text: &str) {
        let widget = TextWidget::from(text);
        self.widget(buf, &widget);
    }
    fn widget<W: Widget>(&mut self, buf: &mut ScreenBuffer, widget: &W) {
        widget.render(buf, self.x, self.current_y);
        self.width = self.width.max(widget.width());
        self.current_y += widget.height() + self.gap;
    }
}
impl Layout for VLayout {
    fn width(&self) -> usize {
        self.width
    }

    fn height(&self) -> usize {
        (self.current_y - self.y).saturating_sub(self.gap).max(0)
    }

    fn init_position_from_other(&mut self, x: usize, y: usize) {
        self.x = x;
        self.y = y;
        self.current_y = y;
    }
    fn init_position_other_layout<L: Layout>(&self, layout: &mut L) {
        layout.init_position_from_other(self.x, self.current_y);
    }

    fn update_self_size_from_other<L: Layout>(&mut self, layout: &L) {
        self.current_y += layout.height() + self.gap;
        self.width = self.width.max(layout.width());
    }
}
struct HLayout {
    x: usize,
    y: usize,
    gap: usize,
    current_x: usize,
    height: usize,
}
impl HLayout {
    fn new(x: usize, y: usize, gap: usize) -> Self {
        Self {
            x,
            y,
            gap,
            current_x: x,
            height: 0,
        }
    }
    fn write_str(&mut self, buf: &mut ScreenBuffer, text: &str) {
        let widget = TextWidget::from(text);
        self.widget(buf, &widget);
    }
    fn widget<W: Widget>(&mut self, buf: &mut ScreenBuffer, widget: &W) {
        widget.render(buf, self.current_x, self.y);
        self.height = self.height.max(widget.height());
        self.current_x += widget.width() + self.gap;
    }
}
impl Layout for HLayout {
    fn width(&self) -> usize {
        (self.current_x - self.x).saturating_sub(self.gap).max(0)
    }

    fn height(&self) -> usize {
        self.height
    }

    fn init_position_from_other(&mut self, x: usize, y: usize) {
        self.current_x = x;
        self.x = x;
        self.y = y;
    }
    fn init_position_other_layout<L: Layout>(&self, layout: &mut L) {
        layout.init_position_from_other(self.current_x, self.y);
    }

    fn update_self_size_from_other<L: Layout>(&mut self, layout: &L) {
        self.current_x += layout.width() + self.gap;
        self.height = self.height.max(layout.height());
    }
}
struct GridLayout {
    x: usize,
    y: usize,
    cols: usize,
    gap_x: usize,
    gap_y: usize,

    current_col: usize,
    current_row: usize,

    col_widths: Vec<usize>,
    row_heights: Vec<usize>,
}
impl GridLayout {
    fn new(x: usize, y: usize, cols: usize, gap_x: usize, gap_y: usize) -> Self {
        Self {
            x,
            y,
            cols,
            gap_x,
            gap_y,
            current_col: 0,
            current_row: 0,
            col_widths: vec![0; cols],
            row_heights: Vec::new(),
        }
    }

    fn current_position(&self) -> (usize, usize) {
        let mut wx = self.x;
        for col in 0..self.current_col {
            wx += self.col_widths[col] + self.gap_x;
        }
        let mut wy = self.y;
        for row in 0..self.current_row {
            wy += self.row_heights[row] + self.gap_y;
        }
        (wx, wy)
    }
    fn widget<W: Widget>(&mut self, buf: &mut ScreenBuffer, widget: &W) {
        let (wx, wy) = self.current_position();
        widget.render(buf, wx, wy);

        // keep max width, height per column, row
        self.col_widths[self.current_col] = self.col_widths[self.current_col].max(widget.width());
        if self.row_heights.len() <= self.current_row {
            self.row_heights.push(widget.height());
        } else {
            self.row_heights[self.current_row] =
                self.row_heights[self.current_row].max(widget.height());
        }

        self.current_col += 1;
        if self.current_col >= self.cols {
            self.current_col = 0;
            self.current_row += 1;
        }
    }

    fn write_str(&mut self, buf: &mut ScreenBuffer, text: &str) {
        let widget = TextWidget::from(text);
        self.widget(buf, &widget);
    }
}
impl Layout for GridLayout {
    fn width(&self) -> usize {
        self.col_widths.iter().sum::<usize>() + self.cols.saturating_sub(1) * self.gap_x
    }

    fn height(&self) -> usize {
        self.row_heights.iter().sum::<usize>()
            + self.row_heights.len().saturating_sub(1) * self.gap_y
    }

    fn init_position_from_other(&mut self, x: usize, y: usize) {
        self.y = y;
        self.x = x;
    }
    fn init_position_other_layout<L: Layout>(&self, layout: &mut L) {
        let (x, y) = self.current_position();
        layout.init_position_from_other(x, y);
    }

    fn update_self_size_from_other<L: Layout>(&mut self, layout: &L) {
        self.col_widths[self.current_col] = self.col_widths[self.current_col].max(layout.width());
        self.row_heights[self.current_row] =
            self.row_heights[self.current_row].max(layout.height());
    }
}

pub trait Widget {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn render(&self, buf: &mut ScreenBuffer, x: usize, y: usize);
}

struct TextWidget<'a> {
    text: &'a str,
}
impl<'a> Widget for TextWidget<'a> {
    fn width(&self) -> usize {
        self.text.len()
    }

    fn height(&self) -> usize {
        1
    }

    fn render(&self, buf: &mut ScreenBuffer, x: usize, y: usize) {
        buf.write_str(x, y, self.text);
    }
}
impl<'a> From<&'a str> for TextWidget<'a> {
    fn from(value: &'a str) -> TextWidget<'a> {
        Self { text: value }
    }
}
#[cfg(test)]
mod test {
    use super::*;
    fn render_test<T: DrawTarget>(ui: &mut Ui<T>) {
        let x_wide = 70;
        ui.available_x = Some(x_wide);
        ui.horizontal(|ui| {
            ui.space(x_wide - 1);
            ui.label("|", None, Align::Left);
        });
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label_align("left left no width", None, Align::Left, Align::Left);
                ui.label_align("left left width", Some(20), Align::Left, Align::Left);
                ui.label_align("right right no width", None, Align::Right, Align::Left);
                ui.label_align("right right width", Some(20), Align::Right, Align::Left);
                ui.label_align("r", None, Align::Right, Align::Left);
            });
            ui.vertical(|ui| {
                ui.label_align("left left no width", None, Align::Left, Align::Left);
                ui.label_align("left left width", Some(20), Align::Left, Align::Left);
                ui.label_align("right right no width", None, Align::Right, Align::Right);
                ui.label_align("right right width", Some(80), Align::Right, Align::Right);
                ui.label_align("r", None, Align::Right, Align::Right);
            });
        });
    }
}
