import json
import unittest
import xml.etree.ElementTree as ET
from observed_problems import canonical, classify

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

if __name__ == '__main__':
    unittest.main()
