use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use chrono::Utc;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CopyExportRole {
    User,
    Agent,
    Cell,
}

impl CopyExportRole {
    fn label(self) -> &'static str {
        match self {
            Self::User => "User",
            Self::Agent => "Agent",
            Self::Cell => "Cell",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CopyExportEntry {
    pub(crate) user_turn_count: usize,
    pub(crate) role: CopyExportRole,
    pub(crate) content: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CopyExportPaths {
    pub(crate) folder: PathBuf,
    pub(crate) xlsx: PathBuf,
    pub(crate) docx: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExtractedTable {
    source_cell: usize,
    rows: Vec<Vec<String>>,
}

pub(crate) fn default_export_folder(cwd: &Path) -> PathBuf {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%SZ");
    cwd.join(format!("thinwedge-copy-export-{timestamp}"))
}

pub(crate) fn export_transcript(
    entries: &[CopyExportEntry],
    folder: &Path,
) -> io::Result<CopyExportPaths> {
    fs::create_dir_all(folder)?;
    let xlsx = folder.join("transcript.xlsx");
    let docx = folder.join("transcript.docx");
    write_xlsx(entries, &xlsx)?;
    write_docx(entries, &docx)?;
    Ok(CopyExportPaths {
        folder: folder.to_path_buf(),
        xlsx,
        docx,
    })
}

fn write_xlsx(entries: &[CopyExportEntry], path: &Path) -> io::Result<()> {
    let tables = extract_tables(entries);
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", options)?;
    write_all(
        &mut zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
"#,
    )?;
    for index in 0..tables.len() {
        write_all(
            &mut zip,
            &format!(
                "  <Override PartName=\"/xl/worksheets/sheet{}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\n",
                index + 2
            ),
        )?;
    }
    write_all(&mut zip, "</Types>")?;

    zip.add_directory("_rels/", options)?;
    zip.start_file("_rels/.rels", options)?;
    write_all(
        &mut zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
    )?;

    zip.add_directory("xl/", options)?;
    zip.start_file("xl/workbook.xml", options)?;
    write_all(
        &mut zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Transcript" sheetId="1" r:id="rId1"/>
"#,
    )?;
    for index in 0..tables.len() {
        let sheet_id = index + 2;
        write_all(
            &mut zip,
            &format!(
                "    <sheet name=\"Table {}\" sheetId=\"{sheet_id}\" r:id=\"rId{sheet_id}\"/>\n",
                index + 1
            ),
        )?;
    }
    write_all(&mut zip, "  </sheets>\n</workbook>")?;

    zip.add_directory("xl/_rels/", options)?;
    zip.start_file("xl/_rels/workbook.xml.rels", options)?;
    write_all(
        &mut zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
"#,
    )?;
    for index in 0..tables.len() {
        let sheet_id = index + 2;
        write_all(
            &mut zip,
            &format!(
                "  <Relationship Id=\"rId{sheet_id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{sheet_id}.xml\"/>\n"
            ),
        )?;
    }
    write_all(&mut zip, "</Relationships>")?;

    zip.add_directory("xl/worksheets/", options)?;
    zip.start_file("xl/worksheets/sheet1.xml", options)?;
    let mut sheet = worksheet_prefix();
    sheet.push_str("    <row r=\"1\">");
    append_inline_cell(&mut sheet, "A1", "Cell");
    append_inline_cell(&mut sheet, "B1", "Role");
    append_inline_cell(&mut sheet, "C1", "Content");
    sheet.push_str("</row>\n");
    for (index, entry) in entries.iter().enumerate() {
        let row = index + 2;
        sheet.push_str(&format!("    <row r=\"{row}\">"));
        append_inline_cell(&mut sheet, &format!("A{row}"), &(index + 1).to_string());
        append_inline_cell(&mut sheet, &format!("B{row}"), entry.role.label());
        append_inline_cell(&mut sheet, &format!("C{row}"), &entry.content);
        sheet.push_str("</row>\n");
    }
    sheet.push_str("  </sheetData>\n</worksheet>");
    write_all(&mut zip, &sheet)?;

    for (index, table) in tables.iter().enumerate() {
        zip.start_file(format!("xl/worksheets/sheet{}.xml", index + 2), options)?;
        let mut sheet = worksheet_prefix();
        sheet.push_str("    <row r=\"1\">");
        append_inline_cell(&mut sheet, "A1", "Source cell");
        append_inline_cell(&mut sheet, "B1", &table.source_cell.to_string());
        sheet.push_str("</row>\n");
        for (row_index, row) in table.rows.iter().enumerate() {
            let excel_row = row_index + 3;
            sheet.push_str(&format!("    <row r=\"{excel_row}\">"));
            for (col_index, value) in row.iter().enumerate() {
                append_inline_cell(&mut sheet, &cell_reference(col_index + 1, excel_row), value);
            }
            sheet.push_str("</row>\n");
        }
        sheet.push_str("  </sheetData>\n</worksheet>");
        write_all(&mut zip, &sheet)?;
    }

    zip.finish()?;
    Ok(())
}

fn worksheet_prefix() -> String {
    String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
"#,
    )
}

fn write_docx(entries: &[CopyExportEntry], path: &Path) -> io::Result<()> {
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", options)?;
    write_all(
        &mut zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )?;

    zip.add_directory("_rels/", options)?;
    zip.start_file("_rels/.rels", options)?;
    write_all(
        &mut zip,
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )?;

    zip.add_directory("word/", options)?;
    zip.start_file("word/document.xml", options)?;
    let mut doc = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>ThinWedge transcript export</w:t></w:r></w:p>
"#,
    );
    for (index, entry) in entries.iter().enumerate() {
        append_docx_paragraph(
            &mut doc,
            &format!("Cell {} - {}", index + 1, entry.role.label()),
        );
        for line in entry.content.lines() {
            append_docx_paragraph(&mut doc, line);
        }
        if entry.content.is_empty() {
            append_docx_paragraph(&mut doc, "");
        }
    }
    doc.push_str("    <w:sectPr/>\n  </w:body>\n</w:document>");
    write_all(&mut zip, &doc)?;

    zip.finish()?;
    Ok(())
}

fn append_inline_cell(out: &mut String, reference: &str, value: &str) {
    out.push_str(&format!(
        r#"<c r="{reference}" t="inlineStr"><is><t xml:space="preserve">{}</t></is></c>"#,
        escape_xml_text(value)
    ));
}

fn cell_reference(mut col: usize, row: usize) -> String {
    let mut letters = Vec::new();
    while col > 0 {
        col -= 1;
        letters.push((b'A' + (col % 26) as u8) as char);
        col /= 26;
    }
    letters.iter().rev().collect::<String>() + &row.to_string()
}

fn extract_tables(entries: &[CopyExportEntry]) -> Vec<ExtractedTable> {
    let mut tables = Vec::new();
    for (entry_index, entry) in entries.iter().enumerate() {
        for rows in extract_markdown_tables(&entry.content)
            .into_iter()
            .chain(extract_tabular_blocks(&entry.content))
        {
            if rows.len() >= 2 && rows.iter().any(|row| row.len() >= 2) {
                tables.push(ExtractedTable {
                    source_cell: entry_index + 1,
                    rows,
                });
            }
        }
    }
    tables
}

fn extract_markdown_tables(content: &str) -> Vec<Vec<Vec<String>>> {
    let lines = content.lines().collect::<Vec<_>>();
    let mut tables = Vec::new();
    let mut index = 0;
    while index + 1 < lines.len() {
        let Some(header) = parse_markdown_table_row(lines[index]) else {
            index += 1;
            continue;
        };
        let Some(separator) = parse_markdown_table_row(lines[index + 1]) else {
            index += 1;
            continue;
        };
        if !is_markdown_separator_row(&separator) || header.len() < 2 {
            index += 1;
            continue;
        }

        let mut rows = vec![header];
        index += 2;
        while index < lines.len() {
            let Some(row) = parse_markdown_table_row(lines[index]) else {
                break;
            };
            if row.len() < 2 {
                break;
            }
            rows.push(row);
            index += 1;
        }
        tables.push(rows);
    }
    tables
}

fn parse_markdown_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return None;
    }
    let trimmed = trimmed.trim_matches('|');
    let cells = trimmed
        .split('|')
        .map(|cell| cell.trim().to_string())
        .collect::<Vec<_>>();
    (cells.len() >= 2).then_some(cells)
}

fn is_markdown_separator_row(cells: &[String]) -> bool {
    cells.iter().all(|cell| {
        let trimmed = cell.trim();
        trimmed.len() >= 3
            && trimmed.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
            && trimmed.contains('-')
    })
}

fn extract_tabular_blocks(content: &str) -> Vec<Vec<Vec<String>>> {
    let mut tables = Vec::new();
    let lines = content.lines().collect::<Vec<_>>();
    let mut index = 0;
    while index < lines.len() {
        if let Some((delimiter, first_row)) = parse_delimited_row(lines[index]) {
            let mut rows = vec![first_row];
            index += 1;
            while index < lines.len() {
                let Some((next_delimiter, row)) = parse_delimited_row(lines[index]) else {
                    break;
                };
                if next_delimiter != delimiter || row.len() != rows[0].len() {
                    break;
                }
                rows.push(row);
                index += 1;
            }
            if rows.len() >= 2 {
                tables.push(rows);
            }
        } else {
            index += 1;
        }
    }
    tables
}

fn parse_delimited_row(line: &str) -> Option<(char, Vec<String>)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.contains('|') {
        return None;
    }
    let delimiter = if trimmed.contains('\t') {
        '\t'
    } else if trimmed.matches(',').count() >= 1 {
        ','
    } else {
        return None;
    };
    let cells = trimmed
        .split(delimiter)
        .map(|cell| cell.trim().to_string())
        .collect::<Vec<_>>();
    (cells.len() >= 2 && cells.iter().all(|cell| !cell.is_empty())).then_some((delimiter, cells))
}

fn append_docx_paragraph(out: &mut String, text: &str) {
    out.push_str("    <w:p><w:r><w:t xml:space=\"preserve\">");
    out.push_str(&escape_xml_text(text));
    out.push_str("</w:t></w:r></w:p>\n");
}

fn escape_xml_text(text: &str) -> String {
    text.chars()
        .filter(|ch| is_xml_compatible(*ch))
        .flat_map(|ch| match ch {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect::<Vec<_>>(),
            '>' => "&gt;".chars().collect::<Vec<_>>(),
            '"' => "&quot;".chars().collect::<Vec<_>>(),
            '\'' => "&apos;".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

fn is_xml_compatible(ch: char) -> bool {
    matches!(
        ch as u32,
        0x09 | 0x0A | 0x0D | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF
    )
}

fn write_all<W: io::Write>(writer: &mut W, text: &str) -> io::Result<()> {
    writer.write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn export_transcript_writes_xlsx_and_docx_zip_payloads() {
        let dir = tempdir().expect("tempdir");
        let entries = vec![
            CopyExportEntry {
                user_turn_count: 1,
                role: CopyExportRole::User,
                content: "Build model <v1>".to_string(),
            },
            CopyExportEntry {
                user_turn_count: 1,
                role: CopyExportRole::Agent,
                content: "Done & verified\nNext line".to_string(),
            },
        ];

        let paths = export_transcript(&entries, dir.path()).expect("export transcript");

        assert!(paths.xlsx.exists());
        assert!(paths.docx.exists());
        let sheet = read_zip_file(&paths.xlsx, "xl/worksheets/sheet1.xml");
        assert!(sheet.contains("Build model &lt;v1&gt;"));
        assert!(sheet.contains("Done &amp; verified"));
        let doc = read_zip_file(&paths.docx, "word/document.xml");
        assert!(doc.contains("Cell 1 - User"));
        assert!(doc.contains("Cell 2 - Agent"));
        assert!(doc.contains("Next line"));
    }

    #[test]
    fn export_transcript_splits_markdown_and_delimited_tables_into_xlsx_sheets() {
        let dir = tempdir().expect("tempdir");
        let entries = vec![CopyExportEntry {
            user_turn_count: 1,
            role: CopyExportRole::Agent,
            content: "\
Here is the table:

| Account | Amount |
| --- | ---: |
| Revenue | 100 |
| Cost | 40 |

And a pasted TSV:

Region\tUnits
US\t12
EU\t8
"
            .to_string(),
        }];

        let paths = export_transcript(&entries, dir.path()).expect("export transcript");

        let workbook = read_zip_file(&paths.xlsx, "xl/workbook.xml");
        assert!(workbook.contains("Table 1"));
        assert!(workbook.contains("Table 2"));
        let markdown_table = read_zip_file(&paths.xlsx, "xl/worksheets/sheet2.xml");
        assert!(markdown_table.contains("Account"));
        assert!(markdown_table.contains("Revenue"));
        assert!(markdown_table.contains("100"));
        let tsv_table = read_zip_file(&paths.xlsx, "xl/worksheets/sheet3.xml");
        assert!(tsv_table.contains("Region"));
        assert!(tsv_table.contains("US"));
        assert!(tsv_table.contains("12"));
    }

    fn read_zip_file(path: &Path, name: &str) -> String {
        let file = File::open(path).expect("open zip");
        let mut archive = zip::ZipArchive::new(file).expect("zip archive");
        let mut file = archive.by_name(name).expect("zip member");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("read member");
        contents
    }
}
