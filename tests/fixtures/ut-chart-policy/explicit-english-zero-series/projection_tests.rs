// Included only by the proposed moxel unit-test patch. Public source-owned
// CF, never an application/ERP blob. No subprocess, native, SQL or temp writes.

fn explicit_english_source_owned_blob() -> Vec<u8> {
    use sha2::Digest as _;
    let bytes: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ut-chart-policy/explicit-english-zero-series/source.cf"
    ));
    assert_eq!(bytes.len(), 7505);
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(bytes)),
        "b2fa2d388face5e7281711d3714815cc304e12bd220f7495be15abfa0a613b8b"
    );
    let archive = ibcmd_cf::archive::decode_packed_archive(
        std::io::Cursor::new(bytes),
        ibcmd_core::limits::ResourceLimits::for_input_bytes(bytes.len() as u64),
        ibcmd_core::artifact::StorageProfileId::parse("storage:test").unwrap(),
    )
    .unwrap();
    archive
        .entries()
        .iter()
        .find(|entry| entry.name() == "30000000-0000-4000-8000-000000000002.0")
        .expect("source-owned spreadsheet row")
        .payload()
        .to_vec()
}

fn explicit_english_source_owned_chart() -> MoxelChart {
    let blob = explicit_english_source_owned_blob();
    let body = crate::compiler::bodies::mxl::decode_compatible_mxl(&blob).unwrap();
    let spreadsheet = parse_moxel_spreadsheet_text(body.native_body_text(), &BTreeMap::new())
        .expect("source-owned spreadsheet must decode");
    let mut charts = spreadsheet
        .drawings
        .into_iter()
        .filter_map(|drawing| match drawing.kind {
            MoxelDrawingKind::Chart(chart) => Some(chart),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(charts.len(), 1);
    charts.pop().unwrap()
}

fn explicit_english_projection_xml(
    chart: &MoxelChart,
    projection: MoxelChartSeriesAutomaticName,
) -> String {
    let mut xml = String::new();
    push_moxel_chart_xml_with_automatic_name(&mut xml, chart, projection);
    xml
}

#[test]
fn explicit_english_projection_republishes_zero_series_extra_name() {
    // Native 8.3.27.2342 observed Pivot/ru for this exact public CF, count0,
    // unchanged name. This proves a selected English projection may not
    // require a real series; it does NOT choose that projection by default.
    let chart = explicit_english_source_owned_chart();
    assert!(chart.real_series.is_empty());
    assert!(!chart.has_extended_scales);
    assert!(!chart.real_extra_series.str_is_changed);
    assert_eq!(chart.real_extra_series.text.len(), 1);
    assert_eq!(chart.real_extra_series.text.first().unwrap().lang, "ru");
    assert_eq!(
        chart.real_extra_series.text.first().unwrap().content,
        "Сводная"
    );
    let stored = explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored);
    let english = explicit_english_projection_xml(
        &chart,
        MoxelChartSeriesAutomaticName::RepublishedInEnglish,
    );
    let cached = "<v8:content>Сводная</v8:content>";
    assert_eq!(stored.matches(cached).count(), 1);
    // Projection invariant, not fabricated native full-XML expected bytes:
    // only this name changes and its stored locale remains ru.
    let expected = stored.replacen(cached, "<v8:content>Pivot</v8:content>", 1);
    assert_eq!(english, expected);
}

#[test]
fn explicit_english_projection_keeps_default_and_stored_identical() {
    let chart = explicit_english_source_owned_chart();
    let mut default = String::new();
    push_moxel_chart_xml(&mut default, &chart);
    assert_eq!(
        MoxelChartSeriesAutomaticName::host(),
        MoxelChartSeriesAutomaticName::Stored
    );
    assert_eq!(
        default,
        explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored)
    );
}

#[test]
fn explicit_english_projection_preserves_user_changed_names() {
    let mut chart = explicit_english_source_owned_chart();
    // Synthetic typed negative controls, not additional native observations.
    chart.real_extra_series.str_is_changed = true;
    chart.real_extra_series.text.first_mut().unwrap().content = "OwnedUserSeries".to_string();
    assert_eq!(
        explicit_english_projection_xml(
            &chart,
            MoxelChartSeriesAutomaticName::RepublishedInEnglish
        ),
        explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored)
    );
}

#[test]
fn explicit_english_projection_leaves_compiler_source_roundtrip_stored() {
    let xml = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ut-chart-policy/wrapper-prototype/tree/CommonTemplates/ChartPolicySeed/Ext/Template.xml"
    ));
    let blob = crate::compiler::bodies::template::compile_evidenced_template(
        crate::compiler::bodies::template::TemplateKind::SpreadsheetDocument,
        crate::compiler::bodies::template::TemplateSource::Spreadsheet {
            xml,
            source: None,
            number_format_hint: None,
        },
    )
    .expect("valid cached-name XML must keep passing the compiler's source guard");
    let roundtrip = try_extract_moxel_spreadsheet_xml(&blob, &BTreeMap::new()).unwrap();
    assert!(roundtrip.contains("<v8:content>Сводная</v8:content>"));
    assert!(!roundtrip.contains("<v8:content>Pivot</v8:content>"));
}

#[test]
fn chart_policy_palette8_extra_series_is_generated_in_explicit_english() {
    let mut chart = explicit_english_source_owned_chart();
    chart.has_extended_scales = true;
    chart.color_palette.name = Some("Palette8");
    let stored = explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored);
    let english = explicit_english_projection_xml(
        &chart,
        MoxelChartSeriesAutomaticName::RepublishedInEnglish,
    );
    assert_eq!(
        english,
        stored.replacen(
            "<v8:content>Сводная</v8:content>",
            "<v8:content>Pivot</v8:content>",
            1
        )
    );
}

#[test]
fn chart_policy_russian_english_stored_and_user_labels_are_isolated() {
    let mut chart = explicit_english_source_owned_chart();
    chart.real_extra_series.text = vec![
        MoxelLocalizedValue {
            lang: "ru".into(),
            content: "Pivot".into(),
        },
        MoxelLocalizedValue {
            lang: "en".into(),
            content: "CachedAutomatic".into(),
        },
    ];
    let stored = explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored);
    let russian = explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Russian);
    let english = explicit_english_projection_xml(
        &chart,
        MoxelChartSeriesAutomaticName::RepublishedInEnglish,
    );
    assert_eq!(
        russian,
        stored
            .replace(
                "<v8:content>Pivot</v8:content>",
                "<v8:content>Сводная</v8:content>"
            )
            .replace(
                "<v8:content>CachedAutomatic</v8:content>",
                "<v8:content>Сводная</v8:content>"
            )
    );
    assert_eq!(
        english,
        stored.replace(
            "<v8:content>CachedAutomatic</v8:content>",
            "<v8:content>Pivot</v8:content>"
        )
    );
    assert_eq!(
        explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored),
        stored
    );
    chart.real_extra_series.str_is_changed = true;
    chart.real_extra_series.text[0].content = "Моя серия & <план>".into();
    chart.real_extra_series.text[1].content = "My series & <plan>".into();
    let user = explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored);
    for policy in [
        MoxelChartSeriesAutomaticName::Russian,
        MoxelChartSeriesAutomaticName::RepublishedInEnglish,
    ] {
        assert_eq!(explicit_english_projection_xml(&chart, policy), user);
    }
}

#[test]
fn chart_policy_named_palette_preserves_real_series_eligibility() {
    let mut chart = explicit_english_source_owned_chart();
    let mut real = explicit_english_source_owned_chart().real_extra_series;
    real.id = 2;
    real.text[0].content = "CachedReal".into();
    chart.real_series.push(real);
    chart.has_extended_scales = true;
    chart.color_palette.name = Some("Palette8");
    let stored = explicit_english_projection_xml(&chart, MoxelChartSeriesAutomaticName::Stored);
    let english = explicit_english_projection_xml(
        &chart,
        MoxelChartSeriesAutomaticName::RepublishedInEnglish,
    );
    assert_eq!(
        english,
        stored.replacen(
            "<v8:content>Сводная</v8:content>",
            "<v8:content>Pivot</v8:content>",
            1
        )
    );
    assert!(english.contains("<v8:content>CachedReal</v8:content>"));
}

#[test]
fn chart_policy_full_decoder_and_nested_gantt_keep_default_stored() {
    let blob = explicit_english_source_owned_blob();
    let refs = BTreeMap::new();
    let stored = try_extract_moxel_spreadsheet_xml(&blob, &refs).unwrap();
    let english = try_extract_moxel_spreadsheet_xml_with_generated_types_and_policy(
        &blob,
        &refs,
        &refs,
        MoxelChartSeriesAutomaticName::RepublishedInEnglish,
    )
    .unwrap();
    assert_eq!(
        english,
        stored.replacen(
            "<v8:content>Сводная</v8:content>",
            "<v8:content>Pivot</v8:content>",
            1
        )
    );
    assert_eq!(
        try_extract_moxel_spreadsheet_xml_with_generated_types_and_policy(
            &blob,
            &refs,
            &refs,
            MoxelChartSeriesAutomaticName::Russian
        )
        .unwrap(),
        stored
    );
    assert_eq!(
        try_extract_moxel_spreadsheet_xml(&blob, &refs).unwrap(),
        stored
    );

    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/native-evidence/8.3.27.2214/moxel-gantt-multiple-levels/object-payload.txt"
    ));
    let mut gantt = parse_moxel_gantt_chart(text, &refs).unwrap();
    gantt.chart.real_extra_series.str_is_changed = false;
    gantt.chart.real_extra_series.text = vec![MoxelLocalizedValue {
        lang: "ru".into(),
        content: "CachedAutomatic".into(),
    }];
    let mut stored = String::new();
    push_moxel_gantt_chart_xml(&mut stored, &gantt);
    for (policy, label) in [
        (MoxelChartSeriesAutomaticName::Russian, "Сводная"),
        (MoxelChartSeriesAutomaticName::RepublishedInEnglish, "Pivot"),
    ] {
        let mut xml = String::new();
        push_moxel_gantt_chart_xml_with_policy(&mut xml, &gantt, policy);
        assert_eq!(
            xml,
            stored.replacen(
                "<v8:content>CachedAutomatic</v8:content>",
                &format!("<v8:content>{label}</v8:content>"),
                1
            )
        );
    }
}
