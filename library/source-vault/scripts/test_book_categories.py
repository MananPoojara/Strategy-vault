import unittest
from book_categories import classify, READING_ONLY_ADDITIONS


class CategoryTests(unittest.TestCase):
    def test_pristine_reading_shelf_has_explicit_unclassified_entries(self):
        self.assertEqual({b['id'] for b in READING_ONLY_ADDITIONS},
                         {'p_intraday_capra','p_tools_tactics','p_tpm1'})
        for book in READING_ONLY_ADDITIONS:
            result=classify(book)
            self.assertEqual(result['categories'],[])
            self.assertEqual(result['certainty'],'unreviewed')
            self.assertFalse(result['performance_verified'])

    def test_discretionary_unknown_and_new_pristine_are_not_forced_mechanical(self):
        for card in [{'id':'jr_loc','title':'The Law of Charts'}, {'id':'p_tools_tactics','title':'Tools and Tactics'},
                     {'id':'unknown','title':'Money, Success and Life'}]:
            r=classify(card);self.assertEqual(r['categories'],[]);self.assertEqual(r['certainty'],'unreviewed')

    def test_source_reviewed_psychology_sizing_can_overlap_without_certification(self):
        r=classify({'id':'champion','title':'Think & Trade Like a Champion'})
        self.assertEqual(r['categories'],['psychology','money_management'])
        self.assertEqual(r['certainty'],'reviewed');self.assertFalse(r['performance_verified'])
        self.assertFalse(r['complete_source_review_claimed']);self.assertTrue(r['evidence'])

    def test_rules_category_is_not_strategy_or_data_readiness(self):
        r=classify({'id':'stc9','title':'Swing Trading College IX'})
        self.assertEqual(r['categories'],['rule_based'])
        self.assertEqual(r['verification_status'],'not_established_by_subject_category')

    def test_catalogue_objective_topic_does_not_gain_source_review_or_performance_claim(self):
        r=classify({'id':'systems','title':'Quantitative Trading Systems','summary':'Three breakout systems; contract data missing.'})
        self.assertEqual(r['categories'],['rule_based']);self.assertEqual(r['certainty'],'catalogue-description')
        self.assertFalse(r['performance_verified'])

    def test_missing_rules_block_mechanical_inference_and_money_in_title_is_not_sizing(self):
        r=classify({'id':'discretion','title':'Trading Systems','summary':'Judged exits, no complete entry/exit rules.'})
        self.assertEqual(r['categories'],[])
        r=classify({'id':'million','title':'How I Made a Million Dollars','summary':'A personal trading narrative.'})
        self.assertEqual(r['categories'],[])

    def test_actual_psychology_money_management_subjects_do_not_force_mechanical_label(self):
        r=classify({'id':'eltfl','title':'Trading for a Living'})
        self.assertEqual(r['categories'],['psychology','money_management']);self.assertEqual(r['certainty'],'reviewed')

    def test_mixed_source_and_catalogue_basis_stays_explicit_per_category(self):
        r=classify({'id':'tharp','title':'Trade Your Way to Financial Freedom'})
        self.assertEqual(r['category_evidence']['rule_based']['certainty'],'catalogue-description')
        self.assertEqual(r['category_evidence']['psychology']['certainty'],'reviewed')
        self.assertEqual(r['category_evidence']['money_management']['certainty'],'reviewed')


if __name__=='__main__':unittest.main()
