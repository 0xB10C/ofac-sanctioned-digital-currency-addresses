#!/usr/bin/env python3

import argparse
import json
import pathlib
import shutil
import sys
import tempfile
import ssl
import urllib.request
import urllib.error
import zipfile
import xml.etree.ElementTree as ET

DEFAULT_SDN_URL = "https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/SDN_ADVANCED.ZIP"
DEFAULT_SDN_FILENAME = "sdn_advanced.xml"

FEATURE_TYPE_TEXT = "Digital Currency Address - "
NAMESPACE = {'sdn': 'https://sanctionslistservice.ofac.treas.gov/api/PublicationPreview/exports/ADVANCED_XML'}

# List of assets that have been sanctioned by the OFAC.
# Possible assets be seen by grepping the sdn_advanced.xml file for "Digital Currency Address".
POSSIBLE_ASSETS = ["XBT", "ETH", "XMR", "LTC", "ZEC", "DASH", "BTG", "ETC",
                   "BSV", "BCH", "XVG", "USDT", "XRP", "ARB", "BSC", "USDC",
                   "TRX", "SOL"]

# List of implemented output formats
OUTPUT_FORMATS = ["TXT", "JSON"]


def build_generate_parser():
    parser = argparse.ArgumentParser(
        description='Tool to extract sanctioned digital currency addresses from the OFAC special designated nationals XML file (sdn_advanced.xml)')
    parser.add_argument('assets', choices=POSSIBLE_ASSETS, nargs='*',
                        default=POSSIBLE_ASSETS[0], help='the asset for which the sanctioned addresses should be extracted (default: XBT (Bitcoin))')
    parser.add_argument('-sdn', '--special-designated-nationals-list', dest='sdn', type=pathlib.Path,
        help='the path to the sdn_advanced.xml file', default=pathlib.Path(DEFAULT_SDN_FILENAME))
    parser.add_argument('-f', '--output-format',  dest='format', nargs='*', choices=OUTPUT_FORMATS,
                        default=OUTPUT_FORMATS[0], help='the output file format of the address list (default: TXT)')
    parser.add_argument('-path', '--output-path', dest='outpath',  type=pathlib.Path, default=pathlib.Path(
        "./data"), help='the path where the lists should be written to (default: current working directory ("./data")')
    return parser


def build_fetch_parser():
    parser = argparse.ArgumentParser(
        description='Download the OFAC special designated nationals XML file (sdn_advanced.xml) from the published ZIP archive')
    parser.add_argument('-o', '--output', dest='output', type=pathlib.Path,
                        default=pathlib.Path(DEFAULT_SDN_FILENAME), help='the path where the XML file should be written (default: ./sdn_advanced.xml)')
    parser.add_argument('--url', dest='url', default=DEFAULT_SDN_URL,
                        help='the URL of the OFAC ZIP archive (default: the published SDN_ADVANCED.ZIP file)')
    return parser


def feature_type_text(asset):
    """returns text we expect in a <FeatureType></FeatureType> tag for a given asset"""
    return "Digital Currency Address - " + asset


def get_address_id(root, asset):
    """returns the feature id of the given asset"""
    feature_type = root.find(
        "sdn:ReferenceValueSets/sdn:FeatureTypeValues/*[.='{}']".format(feature_type_text(asset)), NAMESPACE)
    if feature_type == None:
        raise LookupError("No FeatureType with the name {} found".format(
            feature_type_text(asset)))
    address_id = feature_type.attrib["ID"]
    return address_id


def get_sanctioned_addresses(root, address_id):
    """returns a list of sanctioned addresses for the given address_id"""
    addresses = list()
    for feature in root.findall("sdn:DistinctParties//*[@FeatureTypeID='{}']".format(address_id), NAMESPACE):
        for version_detail in feature.findall(".//sdn:VersionDetail", NAMESPACE):
            addresses.append(version_detail.text)
    return addresses


def write_addresses(addresses, asset, output_formats, outpath):
    if "TXT" in output_formats:
        write_addresses_txt(addresses, asset, outpath)
    if "JSON" in output_formats:
        write_addresses_json(addresses, asset, outpath)


def write_addresses_txt(addresses, asset, outpath):
    output_file = outpath / f"sanctioned_addresses_{asset}.txt"
    with open(output_file, 'w', encoding='utf-8') as out:
        for address in addresses:
            out.write(address+"\n")


def write_addresses_json(addresses, asset, outpath):
    output_file = outpath / f"sanctioned_addresses_{asset}.json"
    with open(output_file, 'w', encoding='utf-8') as out:
        out.write(json.dumps(addresses, indent=2)+"\n")


def fetch_sdn_archive(url, output_path):
    output_path = pathlib.Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory() as tmpdir:
        archive_path = pathlib.Path(tmpdir) / "SDN_ADVANCED.ZIP"
        try:
            response = urllib.request.urlopen(url)
        except urllib.error.URLError as err:
            if not isinstance(getattr(err, "reason", None), ssl.SSLCertVerificationError):
                raise
            print("TLS verification failed; retrying without certificate validation.", file=sys.stderr)
            try:
                response = urllib.request.urlopen(url, context=ssl._create_unverified_context())
            except urllib.error.URLError:
                print(f"FAIL: could not download {url}", file=sys.stderr)
                raise

        with response, open(archive_path, 'wb') as archive_file:
            shutil.copyfileobj(response, archive_file)

        with zipfile.ZipFile(archive_path) as archive:
            xml_members = [member for member in archive.namelist() if member.lower().endswith(".xml")]
            if not xml_members:
                raise LookupError("No XML file found in the downloaded archive")
            with archive.open(xml_members[0]) as source, open(output_path, 'wb') as destination:
                shutil.copyfileobj(source, destination)
        print(f"OK: wrote {output_path}", file=sys.stderr)


def parse_arguments():
    if len(sys.argv) > 1 and sys.argv[1] == "fetch":
        args = build_fetch_parser().parse_args(sys.argv[2:])
        args.command = "fetch"
        return args

    args = build_generate_parser().parse_args()
    args.command = "generate"
    return args


def main():
    args = parse_arguments()

    if args.command == "fetch":
        fetch_sdn_archive(args.url, args.output)
        return

    if not args.sdn.exists():
        raise FileNotFoundError(
            f"{args.sdn} does not exist. Run `python3 generate-address-list.py fetch` first or pass `--sdn /path/to/sdn_advanced.xml`."
        )

    args.outpath.mkdir(parents=True, exist_ok=True)

    tree = ET.parse(args.sdn)
    root = tree.getroot()

    assets = list()
    if type(args.assets) == str:
        assets.append(args.assets)
    else:
        assets = args.assets

    output_formats = list()
    if type(args.format) == str:
        output_formats.append(args.format)
    else:
        output_formats = args.format

    for asset in assets:
        address_id = get_address_id(root, asset)
        addresses = get_sanctioned_addresses(root, address_id)

        # deduplicate addresses
        addresses = list(dict.fromkeys(addresses).keys())
        # sort addresses
        addresses.sort()

        write_addresses(addresses, asset, output_formats, args.outpath)


if __name__ == "__main__":
    main()
