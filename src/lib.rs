use roxmltree::Document;
use serde_json::to_writer_pretty;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs::{self, File};
use std::io::{BufWriter, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

pub const DEFAULT_SDN_URL: &str =
    "https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/SDN_ADVANCED.ZIP";
pub const DEFAULT_SDN_FILENAME: &str = "sdn_advanced.xml";

const FEATURE_TYPE_PREFIX: &str = "Digital Currency Address - ";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Asset {
    Xbt,
    Eth,
    Xmr,
    Ltc,
    Zec,
    Dash,
    Btg,
    Etc,
    Bsv,
    Bch,
    Xvg,
    Usdt,
    Xrp,
    Arb,
    Bsc,
    Usdc,
    Trx,
    Sol,
}

impl Asset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Asset::Xbt => "XBT",
            Asset::Eth => "ETH",
            Asset::Xmr => "XMR",
            Asset::Ltc => "LTC",
            Asset::Zec => "ZEC",
            Asset::Dash => "DASH",
            Asset::Btg => "BTG",
            Asset::Etc => "ETC",
            Asset::Bsv => "BSV",
            Asset::Bch => "BCH",
            Asset::Xvg => "XVG",
            Asset::Usdt => "USDT",
            Asset::Xrp => "XRP",
            Asset::Arb => "ARB",
            Asset::Bsc => "BSC",
            Asset::Usdc => "USDC",
            Asset::Trx => "TRX",
            Asset::Sol => "SOL",
        }
    }

    pub fn from_text(value: &str) -> Option<Self> {
        match value {
            "XBT" => Some(Asset::Xbt),
            "ETH" => Some(Asset::Eth),
            "XMR" => Some(Asset::Xmr),
            "LTC" => Some(Asset::Ltc),
            "ZEC" => Some(Asset::Zec),
            "DASH" => Some(Asset::Dash),
            "BTG" => Some(Asset::Btg),
            "ETC" => Some(Asset::Etc),
            "BSV" => Some(Asset::Bsv),
            "BCH" => Some(Asset::Bch),
            "XVG" => Some(Asset::Xvg),
            "USDT" => Some(Asset::Usdt),
            "XRP" => Some(Asset::Xrp),
            "ARB" => Some(Asset::Arb),
            "BSC" => Some(Asset::Bsc),
            "USDC" => Some(Asset::Usdc),
            "TRX" => Some(Asset::Trx),
            "SOL" => Some(Asset::Sol),
            _ => None,
        }
    }

    pub fn all() -> &'static [Asset] {
        &[
            Asset::Xbt,
            Asset::Eth,
            Asset::Xmr,
            Asset::Ltc,
            Asset::Zec,
            Asset::Dash,
            Asset::Btg,
            Asset::Etc,
            Asset::Bsv,
            Asset::Bch,
            Asset::Xvg,
            Asset::Usdt,
            Asset::Xrp,
            Asset::Arb,
            Asset::Bsc,
            Asset::Usdc,
            Asset::Trx,
            Asset::Sol,
        ]
    }
}

impl Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Txt,
    Json,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Txt => "TXT",
            OutputFormat::Json => "JSON",
        }
    }

    pub fn from_text(value: &str) -> Option<Self> {
        match value {
            "TXT" => Some(OutputFormat::Txt),
            "JSON" => Some(OutputFormat::Json),
            _ => None,
        }
    }

    pub fn all() -> &'static [OutputFormat] {
        &[OutputFormat::Txt, OutputFormat::Json]
    }
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct GenerateArgs {
    pub assets: Vec<Asset>,
    pub sdn: PathBuf,
    pub output_formats: Vec<OutputFormat>,
    pub outpath: PathBuf,
}

#[derive(Debug, Clone)]
pub struct FetchArgs {
    pub output: PathBuf,
    pub url: String,
}

#[derive(Debug, Clone)]
pub enum Command {
    Generate(GenerateArgs),
    Fetch(FetchArgs),
}

#[derive(Debug)]
struct CliError(String);

impl Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for CliError {}

pub fn run(command: Command) -> Result<(), Box<dyn Error>> {
    match command {
        Command::Fetch(args) => fetch_sdn_archive(&args.url, &args.output),
        Command::Generate(args) => generate_address_lists(&args),
    }
}

pub fn parse_args<I>(args: I) -> Result<Command, Box<dyn Error>>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    let mut args = args.into_iter();
    let first = args.next();

    match first.as_ref().and_then(|value| value.to_str()) {
        Some("fetch") => Ok(Command::Fetch(parse_fetch_args(args)?)),
        _ => Ok(Command::Generate(parse_generate_args(
            first.into_iter().chain(args),
        )?)),
    }
}

fn parse_generate_args<I>(args: I) -> Result<GenerateArgs, Box<dyn Error>>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    let mut assets = Vec::new();
    let mut output_formats = Vec::new();
    let mut sdn = PathBuf::from(DEFAULT_SDN_FILENAME);
    let mut outpath = PathBuf::from(".");

    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        let value = arg.to_string_lossy();
        match value.as_ref() {
            "-h" | "--help" => {
                print_generate_help();
                std::process::exit(0);
            }
            "-sdn" | "--special-designated-nationals-list" => {
                sdn = next_path(&mut args, "-sdn")?;
            }
            "-f" | "--output-format" => {
                while let Some(next) = args.peek() {
                    let text = next.to_string_lossy();
                    if text.starts_with('-') {
                        break;
                    }
                    let format = OutputFormat::from_text(&text)
                        .ok_or_else(|| CliError(format!("invalid output format: {text}")))?;
                    output_formats.push(format);
                    args.next();
                }
            }
            "-path" | "--output-path" => {
                outpath = next_path(&mut args, "-path")?;
            }
            other if other.starts_with('-') => {
                return Err(Box::new(CliError(format!("unrecognized option: {other}"))));
            }
            other => {
                let asset = Asset::from_text(other)
                    .ok_or_else(|| CliError(format!("invalid asset: {other}")))?;
                assets.push(asset);
            }
        }
    }

    if assets.is_empty() {
        assets.push(Asset::Xbt);
    }
    if output_formats.is_empty() {
        output_formats.push(OutputFormat::Txt);
    }

    Ok(GenerateArgs {
        assets,
        sdn,
        output_formats,
        outpath,
    })
}

fn parse_fetch_args<I>(args: I) -> Result<FetchArgs, Box<dyn Error>>
where
    I: IntoIterator<Item = std::ffi::OsString>,
{
    let mut output = PathBuf::from(DEFAULT_SDN_FILENAME);
    let mut url = DEFAULT_SDN_URL.to_string();

    let mut args = args.into_iter().peekable();
    while let Some(arg) = args.next() {
        let value = arg.to_string_lossy();
        match value.as_ref() {
            "-h" | "--help" => {
                print_fetch_help();
                std::process::exit(0);
            }
            "-o" | "--output" => {
                output = next_path(&mut args, "-o")?;
            }
            "--url" => {
                url = next_text(&mut args, "--url")?;
            }
            other if other.starts_with('-') => {
                return Err(Box::new(CliError(format!("unrecognized option: {other}"))));
            }
            other => {
                return Err(Box::new(CliError(format!(
                    "unexpected argument for fetch: {other}"
                ))));
            }
        }
    }

    Ok(FetchArgs { output, url })
}

fn next_text<I>(args: &mut std::iter::Peekable<I>, option: &str) -> Result<String, Box<dyn Error>>
where
    I: Iterator<Item = std::ffi::OsString>,
{
    let value = args
        .next()
        .ok_or_else(|| CliError(format!("{option} requires a value")))?;
    Ok(value.to_string_lossy().into_owned())
}

fn next_path<I>(args: &mut std::iter::Peekable<I>, option: &str) -> Result<PathBuf, Box<dyn Error>>
where
    I: Iterator<Item = std::ffi::OsString>,
{
    Ok(PathBuf::from(next_text(args, option)?))
}

fn print_generate_help() {
    println!(
        "Usage: generate-address-list [ASSET ...] [-sdn FILE] [-f TXT JSON] [-path DIR]\n\
         \n\
         Generate sanctioned digital currency address lists from an OFAC XML file.\n\
         \n\
         Assets: {}\n\
         Formats: {}\n\
         \n\
         Fetch the input XML first with: generate-address-list fetch -o sdn_advanced.xml",
        Asset::all()
            .iter()
            .map(Asset::as_str)
            .collect::<Vec<_>>()
            .join(" "),
        OutputFormat::all()
            .iter()
            .map(OutputFormat::as_str)
            .collect::<Vec<_>>()
            .join(" ")
    );
}

fn print_fetch_help() {
    println!(
        "Usage: generate-address-list fetch [-o FILE] [--url URL]\n\
         \n\
         Download the published OFAC SDN XML archive and extract the XML file."
    );
}

pub fn generate_address_lists(args: &GenerateArgs) -> Result<(), Box<dyn Error>> {
    if !args.sdn.exists() {
        return Err(Box::new(CliError(format!(
            "{} does not exist. Run `generate-address-list fetch` first or pass `-sdn /path/to/sdn_advanced.xml`.",
            args.sdn.display()
        ))));
    }

    fs::create_dir_all(&args.outpath)?;
    let xml = fs::read_to_string(&args.sdn)?;
    let doc = Document::parse(&xml)?;

    for asset in &args.assets {
        let address_id = find_address_id(&doc, *asset)?;
        let mut addresses = sanctioned_addresses(&doc, &address_id);
        dedupe_preserve_order(&mut addresses);
        addresses.sort();
        write_addresses(&addresses, *asset, &args.output_formats, &args.outpath)?;
    }

    Ok(())
}

fn find_address_id(doc: &Document<'_>, asset: Asset) -> Result<String, Box<dyn Error>> {
    let target = feature_type_text(asset);

    for node in doc
        .descendants()
        .filter(|node| node.has_tag_name("FeatureType"))
    {
        if node.text().map(str::trim) == Some(target.as_str())
            && node
                .ancestors()
                .any(|ancestor| ancestor.has_tag_name("FeatureTypeValues"))
        {
            if let Some(id) = node.attribute("ID") {
                return Ok(id.to_string());
            }
        }
    }

    Err(Box::new(CliError(format!(
        "No FeatureType with the name {target} found"
    ))))
}

fn sanctioned_addresses(doc: &Document<'_>, address_id: &str) -> Vec<String> {
    let mut addresses = Vec::new();

    for node in doc
        .descendants()
        .filter(|node| node.attribute("FeatureTypeID") == Some(address_id))
    {
        for version_detail in node
            .descendants()
            .filter(|node| node.has_tag_name("VersionDetail"))
        {
            if let Some(text) = version_detail
                .text()
                .map(str::trim)
                .filter(|text| !text.is_empty())
            {
                addresses.push(text.to_string());
            }
        }
    }

    addresses
}

fn dedupe_preserve_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn write_addresses(
    addresses: &[String],
    asset: Asset,
    formats: &[OutputFormat],
    outpath: &Path,
) -> Result<(), Box<dyn Error>> {
    if formats.contains(&OutputFormat::Txt) {
        write_addresses_txt(addresses, asset, outpath)?;
    }
    if formats.contains(&OutputFormat::Json) {
        write_addresses_json(addresses, asset, outpath)?;
    }
    Ok(())
}

fn write_addresses_txt(
    addresses: &[String],
    asset: Asset,
    outpath: &Path,
) -> Result<(), Box<dyn Error>> {
    let output_file = outpath.join(format!("sanctioned_addresses_{}.txt", asset));
    let mut writer = BufWriter::new(File::create(output_file)?);
    for address in addresses {
        writeln!(writer, "{address}")?;
    }
    Ok(())
}

fn write_addresses_json(
    addresses: &[String],
    asset: Asset,
    outpath: &Path,
) -> Result<(), Box<dyn Error>> {
    let output_file = outpath.join(format!("sanctioned_addresses_{}.json", asset));
    let mut writer = BufWriter::new(File::create(output_file)?);
    to_writer_pretty(&mut writer, addresses)?;
    writeln!(writer)?;
    Ok(())
}

pub fn fetch_sdn_archive(url: &str, output_path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }

    let archive_bytes = download_archive(url)?;
    let xml_bytes = extract_first_xml(&archive_bytes)?;
    fs::write(output_path, xml_bytes)?;
    eprintln!("OK: wrote {}", output_path.display());
    Ok(())
}

fn download_archive(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    match download_archive_with_client(url, false) {
        Ok(bytes) => Ok(bytes),
        Err(err) if is_tls_verification_error(err.as_ref()) => {
            eprintln!("TLS verification failed; retrying without certificate validation.");
            match download_archive_with_client(url, true) {
                Ok(bytes) => Ok(bytes),
                Err(err) => {
                    eprintln!("FAIL: could not download {url}");
                    Err(err)
                }
            }
        }
        Err(err) => Err(err),
    }
}

fn download_archive_with_client(url: &str, insecure: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    let client = reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(insecure)
        .build()?;

    let mut response = client.get(url).send()?.error_for_status()?;
    let mut bytes = Vec::new();
    response.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn is_tls_verification_error(err: &(dyn Error + 'static)) -> bool {
    let message = err.to_string().to_lowercase();
    message.contains("certificate verify failed")
        || message.contains("certificate")
        || message.contains("tls")
}

fn extract_first_xml(archive_bytes: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let reader = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(reader)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.name().to_ascii_lowercase().ends_with(".xml") {
            let mut xml = Vec::new();
            entry.read_to_end(&mut xml)?;
            return Ok(xml);
        }
    }

    Err(Box::new(CliError(
        "No XML file found in the downloaded archive".to_string(),
    )))
}

fn feature_type_text(asset: Asset) -> String {
    format!("{FEATURE_TYPE_PREFIX}{}", asset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    #[test]
    fn parses_asset_ids_and_addresses() {
        let xml = r#"
            <Root xmlns="https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/ADVANCED_XML"
                  xmlns:sdn="https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/ADVANCED_XML">
              <sdn:ReferenceValueSets>
                <sdn:FeatureTypeValues>
                  <sdn:FeatureType ID="123">Digital Currency Address - XBT</sdn:FeatureType>
                </sdn:FeatureTypeValues>
              </sdn:ReferenceValueSets>
              <sdn:DistinctParties>
                <sdn:Party FeatureTypeID="123">
                  <sdn:VersionDetail>b</sdn:VersionDetail>
                  <sdn:VersionDetail>a</sdn:VersionDetail>
                  <sdn:VersionDetail>b</sdn:VersionDetail>
                </sdn:Party>
              </sdn:DistinctParties>
            </Root>
        "#;

        let doc = Document::parse(xml).unwrap();
        let id = find_address_id(&doc, Asset::Xbt).unwrap();
        let mut addresses = sanctioned_addresses(&doc, &id);
        dedupe_preserve_order(&mut addresses);
        addresses.sort();

        eprintln!(
            "asset={} feature_type_id={id} addresses={addresses:?}",
            Asset::Xbt
        );

        assert_eq!(id, "123");
        assert_eq!(addresses, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn writes_text_and_json_outputs() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = env::temp_dir().join(format!("ofac-test-{stamp}"));
        fs::create_dir_all(&base).unwrap();

        let addresses = vec!["addr1".to_string(), "addr2".to_string()];
        write_addresses(
            &addresses,
            Asset::Xbt,
            &[OutputFormat::Txt, OutputFormat::Json],
            &base,
        )
        .unwrap();

        eprintln!(
            "wrote {} and {}",
            base.join("sanctioned_addresses_XBT.txt").display(),
            base.join("sanctioned_addresses_XBT.json").display()
        );

        assert_eq!(
            fs::read_to_string(base.join("sanctioned_addresses_XBT.txt")).unwrap(),
            "addr1\naddr2\n"
        );
        assert_eq!(
            fs::read_to_string(base.join("sanctioned_addresses_XBT.json")).unwrap(),
            "[\n  \"addr1\",\n  \"addr2\"\n]\n"
        );

        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn parses_fetch_and_generate_arguments() {
        match parse_args([
            "fetch".into(),
            "-o".into(),
            "custom.xml".into(),
            "--url".into(),
            "https://example.invalid/archive.zip".into(),
        ])
        .unwrap()
        {
            Command::Fetch(args) => {
                eprintln!("fetch output={} url={}", args.output.display(), args.url);
                assert_eq!(args.output, PathBuf::from("custom.xml"));
                assert_eq!(args.url, "https://example.invalid/archive.zip");
            }
            other => panic!("unexpected command: {other:?}"),
        }

        match parse_args([
            "ETH".into(),
            "XBT".into(),
            "-f".into(),
            "JSON".into(),
            "TXT".into(),
            "-sdn".into(),
            "input.xml".into(),
            "-path".into(),
            "out".into(),
        ])
        .unwrap()
        {
            Command::Generate(args) => {
                eprintln!(
                    "generate assets={:?} sdn={} formats={:?} outpath={}",
                    args.assets,
                    args.sdn.display(),
                    args.output_formats,
                    args.outpath.display()
                );
                assert_eq!(args.assets, vec![Asset::Eth, Asset::Xbt]);
                assert_eq!(args.sdn, PathBuf::from("input.xml"));
                assert_eq!(
                    args.output_formats,
                    vec![OutputFormat::Json, OutputFormat::Txt]
                );
                assert_eq!(args.outpath, PathBuf::from("out"));
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn extracts_first_xml_from_zip_archive() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = ZipWriter::new(&mut buffer);
            let options =
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            writer.start_file("notes.txt", options).unwrap();
            writer.write_all(b"ignore me").unwrap();
            writer.start_file("sdn_advanced.xml", options).unwrap();
            writer.write_all(b"<root>ok</root>").unwrap();
            writer.finish().unwrap();
        }

        let xml = extract_first_xml(buffer.get_ref()).unwrap();
        eprintln!("extracted xml bytes={}", xml.len());
        assert_eq!(xml, b"<root>ok</root>");
    }

    #[test]
    fn generates_outputs_for_multiple_assets() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = env::temp_dir().join(format!("ofac-generate-{stamp}"));
        fs::create_dir_all(&base).unwrap();

        let sdn = base.join("sdn_advanced.xml");
        fs::write(
            &sdn,
            r#"
                <Root xmlns="https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/ADVANCED_XML"
                      xmlns:sdn="https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/ADVANCED_XML">
                  <sdn:ReferenceValueSets>
                    <sdn:FeatureTypeValues>
                      <sdn:FeatureType ID="11">Digital Currency Address - XBT</sdn:FeatureType>
                      <sdn:FeatureType ID="22">Digital Currency Address - ETH</sdn:FeatureType>
                    </sdn:FeatureTypeValues>
                  </sdn:ReferenceValueSets>
                  <sdn:DistinctParties>
                    <sdn:Party FeatureTypeID="11">
                      <sdn:VersionDetail>btc-2</sdn:VersionDetail>
                      <sdn:VersionDetail>btc-1</sdn:VersionDetail>
                      <sdn:VersionDetail>btc-1</sdn:VersionDetail>
                    </sdn:Party>
                    <sdn:Party FeatureTypeID="22">
                      <sdn:VersionDetail>eth-1</sdn:VersionDetail>
                    </sdn:Party>
                  </sdn:DistinctParties>
                </Root>
            "#,
        )
        .unwrap();

        let out = base.join("out");
        let args = GenerateArgs {
            assets: vec![Asset::Xbt, Asset::Eth],
            sdn: sdn.clone(),
            output_formats: vec![OutputFormat::Txt, OutputFormat::Json],
            outpath: out.clone(),
        };
        generate_address_lists(&args).unwrap();

        eprintln!(
            "generated files in {} for assets={:?}",
            out.display(),
            args.assets
        );

        assert_eq!(
            fs::read_to_string(out.join("sanctioned_addresses_XBT.txt")).unwrap(),
            "btc-1\nbtc-2\n"
        );
        assert_eq!(
            fs::read_to_string(out.join("sanctioned_addresses_ETH.txt")).unwrap(),
            "eth-1\n"
        );
        assert_eq!(
            fs::read_to_string(out.join("sanctioned_addresses_XBT.json")).unwrap(),
            "[\n  \"btc-1\",\n  \"btc-2\"\n]\n"
        );

        fs::remove_dir_all(&base).unwrap();
    }
}
