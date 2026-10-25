import json
import unittest
import xml.etree.ElementTree as ET
from observed_problems import canonical, classify, parse_xml

NS = 'urn:coverage:form'

def form(value):
    body = '' if value is None else '<Height>' + value + '</Height>'
    return ET.fromstring('<Form xmlns="' + NS + '"><InputField name="Probe">' + body + '</InputField></Form>')

class WitnessContract(unittest.TestCase):
    def setUp(self):
        self.native = form('2')
        self.case = {'id': 'height', 'anchor': {'tag': '{' + NS + '}InputField', 'name': 'Probe'},
                     'property': '{' + NS + '}Height',
                     'native': json.loads(json.dumps(canonical(self.native[0][0]))), 'known_symptom': None}

    def test_retained_widget_with_missing_property_is_the_known_symptom(self):
        self.assertEqual(classify(self.native, form(None), self.case), 'known-symptom')

    def test_an_actual_fix_is_reported_as_equal(self):
        self.assertEqual(classify(self.native, form('2'), self.case), 'equal')

    def test_missing_widget_is_not_a_missing_property_reproduction(self):
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, ET.fromstring('<Form/>'), self.case)

    def test_wrong_value_is_an_unrelated_regression(self):
        with self.assertRaisesRegex(ValueError, 'unrelated mismatch'):
            classify(self.native, form('3'), self.case)

    def test_discarded_native_property_invalidates_the_witness(self):
        with self.assertRaisesRegex(ValueError, 'native no longer retains'):
            classify(form(None), form(None), self.case)

    def test_an_element_in_another_namespace_is_not_the_widget(self):
        wrong = ET.fromstring('<Form><InputField name="Probe"/></Form>')
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, wrong, self.case)

    def test_mask_value_with_added_spaces_is_a_different_value(self):
        native = ET.fromstring('<Form xmlns="' + NS + '"><InputField name="Probe"><Mask>9999999999</Mask></InputField></Form>')
        case = dict(self.case, property='{' + NS + '}Mask',
                    native=json.loads(json.dumps(canonical(native[0][0]))))
        ours = ET.fromstring('<Form xmlns="' + NS + '"><InputField name="Probe"><Mask> 9999999999 </Mask></InputField></Form>')
        with self.assertRaisesRegex(ValueError, 'unrelated mismatch'):
            classify(native, ours, case)

    def test_indentation_between_children_does_not_change_the_condition(self):
        native = ET.fromstring('<Form xmlns="' + NS + '"><InputField name="Probe"><UserVisible>\n  <Common>false</Common>\n</UserVisible></InputField></Form>')
        case = dict(self.case, property='{' + NS + '}UserVisible',
                    native=json.loads(json.dumps(canonical(native[0][0]))))
        ours = ET.fromstring('<Form xmlns="' + NS + '"><InputField name="Probe"><UserVisible><Common>false</Common></UserVisible></InputField></Form>')
        self.assertEqual(classify(native, ours, case), 'equal')

    def test_repeated_property_is_not_accepted(self):
        wrong = form('2')
        wrong[0].append(ET.fromstring('<Height xmlns="' + NS + '">2</Height>'))
        with self.assertRaisesRegex(ValueError, 'repeated property'):
            classify(self.native, wrong, self.case)


CORE = 'http://v8.1c.ru/8.1/data-composition-system/core'
XSI_TYPE = '{http://www.w3.org/2001/XMLSchema-instance}type'


def parameter(name, item_type='dcsset:SettingsParameterValue', settings_namespace='http://v8.1c.ru/8.1/data-composition-system/settings'):
    return parse_xml(('<Settings xmlns:c="' + CORE
                         + '" xmlns:dcsset="' + settings_namespace
                         + '" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">'
                         + '<c:item xsi:type="' + item_type + '"><c:parameter>'
                         + name + '</c:parameter></c:item></Settings>').encode('utf-8'))


class ParameterWitnessContract(unittest.TestCase):
    def setUp(self):
        self.native = parameter('Font')
        self.case = {'id': 'font-name',
                     'anchor': {'tag': '{' + CORE + '}item',
                                'attributes': {XSI_TYPE: '{http://v8.1c.ru/8.1/data-composition-system/settings}SettingsParameterValue'},
                                'child': {'tag': '{' + CORE + '}parameter',
                                          'texts': ['Font', 'Шрифт']}},
                     'property': '{' + CORE + '}parameter',
                     'native': canonical(self.native[0][0]),
                     'known_symptom': canonical(parameter('Шрифт')[0][0])}

    def test_known_parameter_name_change_keeps_the_item(self):
        self.assertEqual(classify(self.native, parameter('Шрифт'), self.case), 'known-symptom')

    def test_corrected_parameter_name_is_equal(self):
        self.assertEqual(classify(self.native, parameter('Font'), self.case), 'equal')

    def test_missing_parameter_item_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, ET.fromstring('<Settings/>'), self.case)

    def test_wrong_parameter_value_is_rejected(self):
        with self.assertRaises(ValueError):
            classify(self.native, parameter('Unexpected'), self.case)

    def test_wrong_item_type_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, parameter('Шрифт', 'dcsset:OtherValue'), self.case)

    def test_rebound_type_namespace_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, parameter('Шрифт', settings_namespace='urn:wrong-settings-type'), self.case)

    def test_equivalent_type_prefix_preserves_identity(self):
        xml = '<Settings xmlns:c="' + CORE + '" xmlns:t="http://v8.1c.ru/8.1/data-composition-system/settings" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><c:item xsi:type="t:SettingsParameterValue"><c:parameter>Шрифт</c:parameter></c:item></Settings>'
        self.assertEqual(classify(self.native, parse_xml(xml.encode()), self.case), 'known-symptom')

    def test_undeclared_type_prefix_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'undeclared xsi:type'):
            parameter('Шрифт', item_type='missing:SettingsParameterValue')

    def test_nested_namespace_rebinding_is_rejected(self):
        xml = '<Settings xmlns:c="' + CORE + '" xmlns:dcsset="http://v8.1c.ru/8.1/data-composition-system/settings" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><c:item xmlns:dcsset="urn:other" xsi:type="dcsset:SettingsParameterValue"><c:parameter>Шрифт</c:parameter></c:item></Settings>'
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, parse_xml(xml.encode()), self.case)

    def test_expanded_name_cannot_be_spoofed_as_lexical_type(self):
        with self.assertRaisesRegex(ValueError, 'invalid xsi:type'):
            parameter('Шрифт', item_type='{http://v8.1c.ru/8.1/data-composition-system/settings}SettingsParameterValue')

    def test_unprefixed_type_uses_the_default_namespace(self):
        root = parse_xml(b'<item xmlns="urn:type" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:type="Value"/>')
        self.assertEqual(root.get(XSI_TYPE), '{urn:type}Value')

    def test_repeated_parameter_item_is_rejected(self):
        ours = parameter('Шрифт')
        ours.append(parameter('Font')[0])
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, ours, self.case)

    def test_repeated_key_inside_one_item_is_rejected(self):
        ours = parameter('Шрифт')
        ours[0].append(parameter('Font')[0][0])
        with self.assertRaisesRegex(ValueError, 'repeated property'):
            classify(self.native, ours, self.case)

    def test_foreign_namespace_does_not_supply_the_key(self):
        ours = parameter('Шрифт')
        ours[0][0].tag = '{urn:other}parameter'
        with self.assertRaisesRegex(ValueError, 'intact widget'):
            classify(self.native, ours, self.case)

    def test_aliases_cannot_accept_another_name(self):
        case = json.loads(json.dumps(self.case))
        case['anchor']['child']['texts'].append('Unexpected')
        with self.assertRaisesRegex(ValueError, 'child identity'):
            classify(self.native, parameter('Unexpected'), case)


if __name__ == '__main__':
    unittest.main()
