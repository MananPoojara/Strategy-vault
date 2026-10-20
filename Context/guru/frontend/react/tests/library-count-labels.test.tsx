import {expect,it} from "vitest";
import {renderToStaticMarkup} from "react-dom/server";
import {HomePage} from "../src/pages/HomePage";
import type {HomeModel,ViewSnapshot} from "../src/state/store";
it("uses plain counter labels and explains grouping without changing supplied counts",()=>{
 const home:HomeModel={market:"in",identifiedAuthors:77,strategiesFound:109,backtestedStrategies:58,progressCurrent:true,implementedMethods:83,enabledVariants:162,implementedBooks:12,curatedAuthors:71,scope:"trading",inventoryGroups:4745,inventoryEntries:35712,authors:1453,entries:3827,families:15,symbol:"Nifty 50",symbolCount:3,basket:false,authorCards:[],bookCards:[],page:{page:1,pages:1,total:0,key:"library_page",label:"authors"},query:"",category:"",view:"authors",indices:[],baskets:[],symbols:[],usAvailable:false,guilloche:"",connorsLink:"#/guru/connors",strategiesLink:"#/strategies"};
 const snapshot:ViewSnapshot={revision:1,loading:false,error:null,html:"",route:null,status:"",state:{},home,reading:null};
 const html=renderToStaticMarkup(<HomePage snapshot={snapshot}/>);
 expect(html).toContain("Total Authors");expect(html).toContain("Total Strategies found");expect(html).toContain("Backtested Strategies");
 expect(html).toContain('data-count="77"');expect(html).toContain('data-count="109"');expect(html).toContain('data-count="58"');
 expect(html).toContain("parameter versions");expect(html).toContain("within each book");
 expect(html).toContain("does not");expect(html).toContain("original book study");
 expect(html).not.toContain("Download book inventory");expect(html).not.toContain(">Larry Connors</a>");expect(html).toContain("Browse all strategies");
 expect(html).not.toContain("Implemented methods");expect(html).not.toContain("Backtest families on");
});
