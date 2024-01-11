#!/usr/bin/env bash
echo "Getting a fresh copy of the SDN list"

curl -Lo sdn_advanced.xml https://www.treasury.gov/ofac/downloads/sanctions/1.0/sdn_advanced.xml

echo "Generating CSV file of aggregated addresses"
python3 generate-address-list.py -a -f CSV

echo "Successfully generated CSV file of aggregated addresses"
echo "Enjoy your day!"