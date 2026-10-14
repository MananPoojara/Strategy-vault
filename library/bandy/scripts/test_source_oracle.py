"""Source-semantic edge fixtures. These are unit inputs, never production data."""
import unittest
from decimal import Decimal as D
from source_oracle import METHODS, READY, cross, exrem, held, smooth, ema, dema, rsi, signals, ledger

class SourceEdges(unittest.TestCase):
 def test_afl_ema_and_rsi_seeds(self):
  self.assertEqual(ema([D(1),D(3),D(8),D(10)],3),[None,None,D(4),D(7)])
  r=rsi([D(10),D(12),D(11),D(13)],2)
  self.assertEqual(r[:3],[None,None,D(50)]);self.assertEqual(r[3],100*D('1.25')/D('1.5'))
 def rows(self,closes):
  return [dict(date=f'2020-01-{i+1:02}',open=D(str(c)),high=D(str(c))+1,low=D(str(c))-1,close=D(str(c))) for i,c in enumerate(closes)]
 def test_cross_ties_and_nulls(self):
  self.assertEqual(cross([None,D(1),D(1),D(2)],[D(1)]*4),[False,False,False,True])
 def test_exrem_resets_and_raw_holding(self):
  self.assertEqual(exrem([True,True,False,True,True],[False,False,True,False,False]),[True,False,False,True,False])
  self.assertEqual(held([True,False,True,False,False],2),[False,False,False,False,True])
 def test_decimal_seed_preserves_constant(self):
  v=dema([D('7.368440908640764337217582396853616784493')]*30,96)
  self.assertFalse(any(cross(v,dema(v,3))))
  self.assertEqual(smooth([D(1),None,D(3)],D('.5')),[D(1),None,D(2)])
 def test_limit_is_next_day_only_and_strict(self):
  rows=self.rows([100,99,98,100,100,103]);rows[4]['low']=D(99);rows[5]['low']=D(98)
  p=dict(fast=1,slow=2,pct=1);s=signals('hbqts_limit_entry',rows,p)
  self.assertTrue(s[3]['raw_buy']);self.assertFalse(s[4]['buy']);self.assertFalse(s[5]['buy'])
  self.assertEqual(ledger('hbqts_limit_entry',rows,4,5,p,s),[])
  rows[4]['low']=D('98.99');t=ledger('hbqts_limit_entry',rows,4,5,p)
  self.assertEqual((t[0]['entry'],t[0]['entry_px'],t[0]['entry_kind']),(4,D(99),'limit'))
 def test_profit_source_loop_and_strict_target(self):
  rows=self.rows([100,99,98,100,100,100]);rows[3]['high']=D(120);rows[4]['high']=D(110);rows[5]['high']=D('110.01')
  p=dict(fast=1,slow=2,tgt=10);s=signals('hbqts_profit_target',rows,p)
  self.assertFalse(s[3]['sell']);self.assertFalse(s[4]['sell']);self.assertTrue(s[5]['sell'])
  self.assertEqual(ledger('hbqts_profit_target',rows,4,5,p,s),[])
  t=ledger('hbqts_profit_target',rows,0,5,p,s);self.assertEqual(t[0]['exit'],5)
 def test_limit_can_exit_at_close_after_same_day_fill(self):
  rows=self.rows([100,99,98,100,97]);rows[4]['high']=D(100);rows[4]['low']=D(96)
  p=dict(fast=1,slow=2,pct=1);s=signals('hbqts_limit_entry',rows,p)
  self.assertTrue(s[4]['buy'] and s[4]['sell'])
  t=ledger('hbqts_limit_entry',rows,0,4,p,s)
  self.assertEqual((t[0]['entry'],t[0]['exit'],t[0]['entry_px'],t[0]['exit_px'],t[0]['open']),(4,4,D(99),D(97),False))
 def test_terminal_entry_and_negative_open_mark(self):
  rows=self.rows([100,100,100,90]);p=dict(every=2,hold=5)
  t=ledger('hbqts_fixed_interval',rows,1,3,p)
  self.assertTrue(t[0]['open']);self.assertEqual(t[0]['entry'],2);self.assertLess(t[0]['exit_px'],t[0]['entry_px'])
  p=dict(every=3,hold=1);t=ledger('hbqts_fixed_interval',rows,1,3,p)
  self.assertEqual(t[0]['entry'],3);self.assertTrue(t[0]['open'])
 def test_flat_rsi_denominator_has_no_signal(self):
  s=signals('hbqts_stoch_rsi',self.rows([100]*80),METHODS['hbqts_stoch_rsi']['params'])
  self.assertFalse(any(v['buy'] or v['sell'] for v in s))
 def test_only_certifiable_subset_is_ready(self):
  self.assertEqual(len(READY),13);self.assertNotIn('hbqts_stop_entry',READY);self.assertNotIn('hbqts_apz',READY)

if __name__=='__main__':unittest.main()
