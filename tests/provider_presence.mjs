import assert from 'node:assert/strict';
import {alignPresenceScores,appendGraphOutputs,installPresenceScores,normalizePresenceFlags} from '../tools/synthetic/provider_presence.mjs';
assert.deepEqual(normalizePresenceFlags([0,1,false,true]),[false,true,false,true]);
for(const value of ['1',2,null,NaN])assert.throws(()=>normalizePresenceFlags([value]),/Invalid native/);
const a=Uint8Array.of(10,1,7),b=Uint8Array.of(10,1,8);
const at=values=>({values,timestamp:12});
const source={scores:at([.81,.12,.91]),flags:at([true,false,true]),raw:at([a,b]),final:at([b,a])};
// Filtered-out ROIs and a reversed final array cannot transfer one hand's score.
// The production graph supports at most two ROI attempts, so use two in that
// case and independently test a failed first ROI below.
const both={...source,scores:at([.81,.91]),flags:at([true,true])};
assert.deepEqual(alignPresenceScores(both,12,2).scores,[.91,.81]);
assert.deepEqual(alignPresenceScores({...both,final:at([b])},12,1).scores,[.91]);
assert.deepEqual(alignPresenceScores({scores:at([.12,.91]),flags:at([false,true]),raw:at([b]),final:at([b])},12,1).scores,[.91]);
assert.deepEqual(alignPresenceScores({...both,raw:at([a,a]),final:at([a])},12,1),{scores:[null],reason:'presence_correspondence'});
assert.deepEqual(alignPresenceScores({...both,final:at([a,a])},12,2),{scores:[null,null],reason:'presence_correspondence'});
assert.deepEqual(alignPresenceScores({...both,final:at([Uint8Array.of(7)])},12,1).scores,[null]);
assert.deepEqual(alignPresenceScores(both,13,2),{scores:[null,null],reason:'presence_timestamp'});
assert.deepEqual(alignPresenceScores({},12,1).scores,[null]);
for(const scores of [[.9],[NaN,.9],[1.1,.9],[.4,.9]])assert.equal(alignPresenceScores({...both,scores:at(scores)},12,2).reason,'presence_shape');
assert.equal(alignPresenceScores(source,12,2).reason,'presence_shape');
assert.deepEqual(alignPresenceScores({},12,0),{scores:[],reason:null});
assert.throws(()=>alignPresenceScores({},12,3),/Invalid/);
// Normative protobuf field15 tag/length and an existing field remain exact.
assert.deepEqual(appendGraphOutputs(Uint8Array.of(8,1),['abc']),Uint8Array.of(8,1,122,3,97,98,99));
const long=appendGraphOutputs(new Uint8Array(),['a'.repeat(128)]);
assert.deepEqual([...long.slice(0,3)],[122,128,1]);assert.equal(long.length,131);
for(const streams of [[],['same','same'],['bad:stream'],['x'.repeat(1025)]])assert.throws(()=>appendGraphOutputs(new Uint8Array(),streams));
await assert.rejects(installPresenceScores({},'unqualified'),/Unsupported/);
console.log('Presence correspondence, filtering, ambiguity, freshness and bounded graph export passed');
