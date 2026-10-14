import {expect, it} from 'vitest';
import {renderToStaticMarkup} from 'react-dom/server';
import {AuthorCard} from '../src/components/AuthorCard';
import type {AuthorCard as Model} from '../src/state/store';

it('names Bandy stock results and exposes their real stock route without claiming Nifty availability', () => {
  const href = '#/guru/bandy?region=in&basket=nifty50&tk=basket%3Anifty50%3ARELIANCE&from=2010-01-01&to=2020-09-11';
  const model: Model = {id:'bandy',name:'Howard Bandy',curated:true,href,photo:'',initials:'HB',tagline:'Quantitative rules',preview:'Quantitative Trading Systems',entries:1,catalogue:1,reading:0,families:40,available:11,availableContext:'RELIANCE · stock · India · 2010-01-01 to 2020-09-11',contextChanged:true,openLabel:'Open stock backtests',status:'available',statusTitle:'11 available families',statusText:'Measured stock results; original study not certified.',sharpe:null};
  const html = renderToStaticMarkup(<AuthorCard author={model} index={0} />);
  expect(html).toContain('data-backtest-context="bandy"');
  expect(html).toContain('RELIANCE · stock · India · 2010-01-01 to 2020-09-11');
  expect(html).toContain('Open stock backtests');
  expect(html).toContain('no results on the selected test data');
  expect(html).toContain(href.replaceAll('&','&amp;'));
  expect(html).not.toContain('Nifty backtest');
});
