use plotters::prelude::*;

pub fn make_porkchop_plot(
    dv_grid: &[Vec<f32>],
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1) find global min/max
    let (vmin, vmax) = {
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for row in dv_grid {
            for &dv in row {
                min = min.min(dv);
                max = max.max(dv);
            }
        }
        (min, max)
    };

    // 2) prepare the drawing area
    let root = BitMapBackend::new("porkchop.png", (width, height))
        .into_drawing_area();
    root.fill(&WHITE)?;

    // 3) build the chart with u32 axes
    let mut chart = ChartBuilder::on(&root)
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0u32..width, 0u32..height)?;
    chart
        .configure_mesh()
        .x_desc("Departure (days from t=0)")
        .y_desc("Flight Time (days)")
        .draw()?;

    // Draw each pixel
    for (row_idx, row) in dv_grid.iter().enumerate() {
        for (col_idx, &dv) in row.iter().enumerate() {

            let t = ((dv - vmin) / (vmax - vmin)).clamp(0.0, 1.0);
            let color = RGBColor(
                (t * 255.0) as u8,
                0,
                ((1.0 - t) * 255.0) as u8,
            );

            let x0 = col_idx as u32;
            let y0 = height - (row_idx as u32) - 1;
            let x1 = x0 + 1;
            let y1 = y0 + 1;

            chart.draw_series(std::iter::once(
                Rectangle::new([(x0, y0), (x1, y1)], color.filled())
            ))?;
        }
    }

    Ok(())
}
