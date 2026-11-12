#!/usr/bin/env python3
"""Regression checks for fail-closed native protocol generation."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("inventory", ROOT / "tools/migration-inventory.py")
inventory = importlib.util.module_from_spec(spec)
spec.loader.exec_module(inventory)
SOURCE = (ROOT / "protocol/src/lib.rs").read_text()


class InventoryTests(unittest.TestCase):
    def test_sensitive_policy_and_schema(self):
        methods, capabilities, metadata = inventory.inventory(SOURCE)
        self.assertEqual(len(methods), 155)
        self.assertEqual(capabilities["AppUninstall"], "AppManagement")
        self.assertEqual(capabilities["AppUninstallPlan"], "AppManagement")
        self.assertEqual(capabilities["AppUninstallPrompt"], "AppManagement")
        self.assertEqual(capabilities["SettingsUsers"], "SystemAdministration")
        self.assertEqual(metadata["surfaces"]["locker"], ["Unlock"])
        self.assertNotIn("AppManagement", metadata["surfaces"]["options"])
        self.assertEqual(metadata["types"]["ProviderId"]["values"], ["apps", "settings", "calculator"])
        self.assertTrue(metadata["types"]["SearchQuery"]["closed"])
        self.assertFalse(metadata["types"]["SearchQuery"]["fields"][0]["optional"])
        self.assertTrue(all(f["optional"] for f in metadata["types"]["TaskbarPreferences"]["fields"]))

    def test_new_variant_without_policy_is_rejected(self):
        changed = SOURCE.replace("pub enum Request {", 'pub enum Request {\n    #[serde(rename = "test.new")]\n    TestNew,')
        with self.assertRaisesRegex(ValueError, "capability mismatch"):
            inventory.inventory(changed)

    def test_unknown_parameter_type_is_rejected(self):
        changed = SOURCE.replace("SearchQuery(SearchQuery),", "SearchQuery(UnknownType),")
        with self.assertRaisesRegex(ValueError, "Missing native schema"):
            inventory.inventory(changed)


if __name__ == "__main__":
    unittest.main()
