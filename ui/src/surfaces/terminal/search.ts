import type { Terminal } from "../../vendor/xterm";
export type Match = {row:number; start:number; length:number};
// Isolate optional regular expressions so a pathological pattern cannot freeze the UI.
const workerSource = `onmessage = ({data}) => {
 try {
  const pattern = data.regex ? new RegExp(data.query, data.caseSensitive ? "gu" : "giu") : null;
  const needle = data.caseSensitive ? data.query : data.query.toLowerCase();
  const matches=[];
  for (const line of data.lines) {
   if (pattern) {
    pattern.lastIndex=0; let match;
    while ((match=pattern.exec(line.text)) && matches.length<5000) {
     if (match[0].length) matches.push({row:line.row,start:match.index,length:match[0].length});
     else pattern.lastIndex += String.fromCodePoint(line.text.codePointAt(pattern.lastIndex)||32).length;
    }
   } else {
    const text=data.caseSensitive ? line.text : line.text.toLowerCase(); let offset=0,index;
    while ((index=text.indexOf(needle,offset))!==-1 && matches.length<5000) { matches.push({row:line.row,start:index,length:data.query.length}); offset=index+Math.max(1,data.query.length); }
   }
   if(matches.length>=5000) break;
  }
  postMessage({matches});
 } catch(error) {postMessage({error:String(error.message||error)});}
};`;
export function searchBuffer(term:Terminal, query:string, caseSensitive:boolean, regex:boolean): {promise:Promise<Match[]>;cancel:()=>void} {
  const lines:{row:number;text:string}[]=[];
  for(let y=0;y<term.buffer.active.length;y++) {
    const line=term.buffer.active.getLine(y); if(!line) continue;
    const text=line.translateToString(!term.buffer.active.getLine(y+1)?.isWrapped);
    if(line.isWrapped && lines.length) lines[lines.length-1]!.text+=text;
    else lines.push({row:y,text});
  }
  const url=URL.createObjectURL(new Blob([workerSource],{type:"text/javascript"}));
  const worker=new Worker(url); URL.revokeObjectURL(url);
  let timer:ReturnType<typeof setTimeout>;
  const cancel=()=>{clearTimeout(timer);worker.terminate();};
  const promise=new Promise<Match[]>((resolve,reject)=>{
    timer=setTimeout(()=>{cancel();reject(new Error("Search took too long. Use a simpler expression."));},500);
    worker.onmessage=event=>{cancel();event.data.error ? reject(new Error(event.data.error)) : resolve(event.data.matches);};
    worker.onerror=()=>{cancel();reject(new Error("Search worker is unavailable."));};
    worker.postMessage({lines,query,caseSensitive,regex});
  });
  return {promise,cancel};
}
export function revealMatch(term:Terminal, match:Match) {
  let offset=0,start:{x:number;y:number}|undefined,end:{x:number;y:number}|undefined;
  for(let y=match.row;y<term.buffer.active.length;y++) {
    const line=term.buffer.active.getLine(y); if(!line || y>match.row && !line.isWrapped) break;
    const length=line.translateToString(!term.buffer.active.getLine(y+1)?.isWrapped).length; let rowOffset=0;
    for(let x=0;x<term.cols && rowOffset<length;x++) {
      const cell=line.getCell(x); if(!cell || cell.getWidth()===0) continue;
      const chars=cell.getChars()||" ";
      if(!start && offset+rowOffset+chars.length>match.start) start={x,y};
      rowOffset+=chars.length;
      if(start && offset+rowOffset>=match.start+match.length) {end={x:x+Math.max(1,cell.getWidth()),y};break;}
    }
    if(end) break; offset+=length;
  }
  if(start && end) {term.select(start.x,start.y,(end.y-start.y)*term.cols+end.x-start.x);term.scrollToLine(Math.max(0,start.y-2));}
}
