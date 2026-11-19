import {expect,it} from "vitest";
import {renderToStaticMarkup} from "react-dom/server";
import {HomePage} from "../src/pages/HomePage";
import type {HomeModel,ViewSnapshot} from "../src/state/store";
import {shareReview} from "../src/services/source-review-data";
it("uses plain counter labels without changing supplied counts; no share-file notes on the home page (user, 18 Nov)",()=>{
 const home:HomeModel={market:"in",identifiedAuthors:77,strategiesFound:109,backtestedStrategies:58,progressCurrent:true,implementedMethods:83,enabledVariants:162,implementedBooks:12,curatedAuthors:71,scope:"trading",inventoryGroups:4745,inventoryEntries:35712,authors:1453,entries:3827,families:15,symbol:"Nifty 50",symbolCount:3,basket:false,authorCards:[],bookCards:[],page:{page:1,pages:1,total:0,key:"library_page",label:"authors"},query:"",category:"",view:"authors",indices:[],baskets:[],symbols:[],usAvailable:false,guilloche:"",connorsLink:"#/guru/connors",strategiesLink:"#/strategies"};
 const snapshot:ViewSnapshot={revision:1,loading:false,error:null,html:"",route:null,status:"",state:{},home,reading:null};
 const html=renderToStaticMarkup(<HomePage snapshot={snapshot}/>);
 expect(html).not.toContain("still calculating");
 expect(renderToStaticMarkup(<HomePage snapshot={{...snapshot,home:{...home,symbol:"NIFTY 50",resultsPending:true}}}/>)).toContain("Backtest details for NIFTY 50 are still calculating");
 expect(html).toContain("Trading authors (estimate)");expect(html).toContain("Total Strategies");expect(html).toContain("Backtested Strategies");
 expect(html).not.toContain('data-count="77"');expect(html).toContain('data-count="109"');expect(html).toContain('data-count="58"');
 expect(html).not.toContain("Download book inventory");expect(html).not.toContain(">Larry Connors</a>");expect(html).toContain("Browse all strategies");
 expect(html).not.toContain("Implemented methods");expect(html).not.toContain("Backtest families on");
 // Whole-share reviewed totals are shown; the converted subset appears in the note only when supplied.
 expect(html).toContain(`<dt>Trading authors (estimate)</dt><dd data-count="${shareReview.authors}">${shareReview.authors}</dd><dd class="count-asof">as of 16 Nov 2026</dd>`);
 expect(html).toContain(`<dt>Trading books (estimate)</dt><dd data-count="${shareReview.books}">${shareReview.books}</dd><dd class="count-asof">as of 16 Nov 2026</dd>`);
 // User (18 Nov 2026): the explanatory notes and the Research share files strip are removed from the home page.
 expect(html).not.toContain("These estimates cover");expect(html).not.toContain("Research share files");expect(html).not.toContain("same name and size");
});
