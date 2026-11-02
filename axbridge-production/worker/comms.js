import { auth, audit, body, bool, communityMember, cursor, empty, fail, id, int, json, member, now, oneOf, optionalText, queryInt, rate, text } from "./core.js";
const messageTypes=["text","voice","morse","sign","music","image","video","audio","location","alarm","reaction","file"];
export async function conversations(request,env,match) {
  const u=await auth(env,request);
  if(!match && request.method==="GET") { const r=await env.AL_DB.prepare("SELECT c.*,p.role,p.pinned,p.muted,p.archived,p.last_read_at FROM conversations c JOIN conversation_participants p ON p.conversation_id=c.id WHERE p.user_id=? AND p.left_at IS NULL ORDER BY p.pinned DESC,c.updated_at DESC LIMIT 100").bind(u.id).all(); return json(env,request,{conversations:r.results}); }
  if(!match && request.method==="POST") {
    const b=await body(request), type=oneOf(b.type,"type",["direct","group"],"direct"), ids=[...new Set(b.participantIds||[])];
    if(!Array.isArray(b.participantIds)||ids.some(x=>typeof x!=="string")) fail(422,"VALIDATION","participantIds must be an array");
    if(type==="direct"&&ids.length!==1) fail(422,"VALIDATION","Direct conversation requires one participant");
    if(type==="group"&&(ids.length<1||ids.length>49)) fail(422,"VALIDATION","Groups support 2–50 participants");
    const users=await env.AL_DB.prepare(`SELECT id FROM users WHERE id IN (${ids.map(()=>"?").join(",")}) AND deleted_at IS NULL`).bind(...ids).all();
    if(users.results.length!==ids.length) fail(404,"NOT_FOUND","Participant not found");
    if(type==="direct") { const existing=await env.AL_DB.prepare("SELECT p1.conversation_id FROM conversation_participants p1 JOIN conversation_participants p2 ON p1.conversation_id=p2.conversation_id JOIN conversations c ON c.id=p1.conversation_id WHERE c.type='direct' AND p1.user_id=? AND p2.user_id=?").bind(u.id,ids[0]).first(); if(existing) return json(env,request,{conversationId:existing.conversation_id}); }
    const cid=id(),stamp=now(),title=type==="group"?text(b.title,"title",2,100):null;
    await env.AL_DB.batch([env.AL_DB.prepare("INSERT INTO conversations(id,type,title,created_by,created_at,updated_at) VALUES(?,?,?,?,?,?)").bind(cid,type,title,u.id,stamp,stamp),...[u.id,...ids].map(uid=>env.AL_DB.prepare("INSERT INTO conversation_participants(conversation_id,user_id,role,joined_at) VALUES(?,?,?,?)").bind(cid,uid,uid===u.id?"owner":"member",stamp))]);
    await audit(env,u.id,"conversation.create","conversation",cid); return json(env,request,{conversationId:cid},201);
  }
  const cid=match[1], membership=await member(env,cid,u.id);
  if(match[2]==="messages") return messages(request,env,u,cid,match[3]);
  if(match[2]==="participants"&&request.method==="POST") {
    if(!["owner","admin"].includes(membership.role)) fail(403,"FORBIDDEN","Conversation admin required");
    const b=await body(request), count=await env.AL_DB.prepare("SELECT COUNT(*) c FROM conversation_participants WHERE conversation_id=? AND left_at IS NULL").bind(cid).first(); if(count.c>=50) fail(409,"GROUP_FULL","Group is full");
    await env.AL_DB.prepare("INSERT INTO conversation_participants(conversation_id,user_id,role,joined_at) VALUES(?,?,'member',?)").bind(cid,text(b.userId,"userId",10,100),now()).run(); return empty(env,request,201);
  }
  if(request.method==="PATCH") { const b=await body(request); await env.AL_DB.prepare("UPDATE conversation_participants SET pinned=?,muted=?,archived=? WHERE conversation_id=? AND user_id=?").bind(b.pinned==null?membership.pinned:bool(b.pinned),b.muted==null?membership.muted:bool(b.muted),b.archived==null?membership.archived:bool(b.archived),cid,u.id).run(); return empty(env,request); }
  if(request.method==="DELETE") { await env.AL_DB.prepare("UPDATE conversation_participants SET left_at=? WHERE conversation_id=? AND user_id=?").bind(now(),cid,u.id).run(); return empty(env,request); }
  fail(405,"METHOD_NOT_ALLOWED","Method not allowed");
}
async function messages(request,env,u,cid,messageId) {
  await member(env,cid,u.id);
  if(!messageId&&request.method==="GET") { const limit=queryInt(new URL(request.url),"limit",50,1,100),before=cursor(new URL(request.url)); const r=await env.AL_DB.prepare("SELECT id,conversation_id,sender_id,type,ciphertext,nonce,media_key,reply_to_id,client_timestamp,created_at,edited_at,expires_at FROM messages WHERE conversation_id=? AND created_at<? AND deleted_at IS NULL ORDER BY created_at DESC LIMIT ?").bind(cid,before,limit+1).all(); const rows=r.results; return json(env,request,{messages:rows.slice(0,limit),nextBefore:rows.length>limit?rows[limit-1].created_at:null}); }
  if(!messageId&&request.method==="POST") {
    await rate(env,u.id,"message",120,60); const b=await body(request,131072),mid=text(b.id,"id",10,100),type=oneOf(b.type,"type",messageTypes,"text"),ciphertext=text(b.ciphertext,"ciphertext",1,90000),nonce=text(b.nonce,"nonce",8,256);
    if(b.replyToId) { const reply=await env.AL_DB.prepare("SELECT id FROM messages WHERE id=? AND conversation_id=?").bind(b.replyToId,cid).first(); if(!reply) fail(422,"INVALID_REPLY","Reply target not found"); }
    const clientTs=optionalText(b.clientTimestamp,"clientTimestamp",40)||now(), stamp=now();
    try { await env.AL_DB.batch([env.AL_DB.prepare("INSERT INTO messages(id,conversation_id,sender_id,type,ciphertext,nonce,media_key,reply_to_id,client_timestamp,created_at,expires_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)").bind(mid,cid,u.id,type,ciphertext,nonce,optionalText(b.mediaKey,"mediaKey",500),optionalText(b.replyToId,"replyToId",100),clientTs,stamp,optionalText(b.expiresAt,"expiresAt",40)),env.AL_DB.prepare("UPDATE conversations SET updated_at=? WHERE id=?").bind(stamp,cid)]); } catch(e) { if(String(e).includes("UNIQUE")) { const existing=await env.AL_DB.prepare("SELECT id FROM messages WHERE id=? AND sender_id=?").bind(mid,u.id).first(); if(existing) return json(env,request,{id:mid,status:"sent",duplicate:true}); } throw e; }
    return json(env,request,{id:mid,status:"sent",createdAt:stamp},201);
  }
  if(messageId==="receipts"&&request.method==="POST") { const b=await body(request),status=oneOf(b.status,"status",["delivered","read"]); await env.AL_DB.prepare("INSERT INTO message_receipts(message_id,user_id,status,updated_at) VALUES(?,?,?,?) ON CONFLICT(message_id,user_id) DO UPDATE SET status=excluded.status,updated_at=excluded.updated_at").bind(text(b.messageId,"messageId",10,100),u.id,status,now()).run(); return empty(env,request); }
  const m=await env.AL_DB.prepare("SELECT * FROM messages WHERE id=? AND conversation_id=?").bind(messageId,cid).first(); if(!m) fail(404,"NOT_FOUND","Message not found");
  if(request.method==="DELETE") { if(m.sender_id!==u.id) fail(403,"FORBIDDEN","Only sender can delete"); await env.AL_DB.prepare("UPDATE messages SET deleted_at=?,ciphertext='',media_key=NULL WHERE id=?").bind(now(),messageId).run(); return empty(env,request); }
  if(request.method==="PATCH") { const b=await body(request); await env.AL_DB.prepare("INSERT INTO message_reactions(message_id,user_id,reaction,created_at) VALUES(?,?,?,?) ON CONFLICT(message_id,user_id) DO UPDATE SET reaction=excluded.reaction").bind(messageId,u.id,text(b.reaction,"reaction",1,32),now()).run(); return empty(env,request); }
  fail(405,"METHOD_NOT_ALLOWED","Method not allowed");
}
export async function media(request,env,match) {
  const u=await auth(env,request);
  if(!match&&request.method==="POST") {
    await rate(env,u.id,"upload",30,3600); const length=Number(request.headers.get("content-length")||0); if(length<1||length>25*1024*1024) fail(413,"MEDIA_SIZE","Encrypted media must be 1 byte–25 MiB");
    if(request.headers.get("content-type")!=="application/octet-stream") fail(415,"ENCRYPTED_MEDIA_REQUIRED","Only encrypted binary envelopes are accepted");
    const key=`users/${u.id}/media/${id()}`, stamp=now(); await env.AL_BUCKET.put(key,request.body,{httpMetadata:{contentType:"application/octet-stream"},customMetadata:{owner:u.id}});
    await env.AL_DB.prepare("INSERT INTO media_objects(id,owner_id,r2_key,size_bytes,sha256,created_at) VALUES(?,?,?,?,?,?)").bind(id(),u.id,key,length,request.headers.get("x-content-sha256")||null,stamp).run();
    return json(env,request,{mediaKey:key,size:length},201);
  }
  const key=decodeURIComponent(match[1]); if(!key.startsWith(`users/${u.id}/media/`)) fail(403,"FORBIDDEN","Media access denied");
  if(request.method==="GET") { const obj=await env.AL_BUCKET.get(key); if(!obj) fail(404,"NOT_FOUND","Media not found"); return new Response(obj.body,{headers:{"Content-Type":"application/octet-stream","Content-Length":String(obj.size),"Cache-Control":"private, no-store","X-Content-Type-Options":"nosniff"}}); }
  if(request.method==="DELETE") { await env.AL_BUCKET.delete(key); await env.AL_DB.prepare("DELETE FROM media_objects WHERE owner_id=? AND r2_key=?").bind(u.id,key).run(); return empty(env,request); }
  fail(405,"METHOD_NOT_ALLOWED","Method not allowed");
}
export async function emergency(request,env,match) {
  const u=await auth(env,request);
  if(!match&&request.method==="GET") { const r=await env.AL_DB.prepare("SELECT * FROM emergency_events WHERE initiator_id=? ORDER BY created_at DESC LIMIT 100").bind(u.id).all(); return json(env,request,{events:r.results}); }
  if(!match&&request.method==="POST") {
    const b=await body(request); if(b.confirmed!==true) fail(422,"CONFIRMATION_REQUIRED","Alarm confirmation required");
    const duration=int(b.durationMinutes,"durationMinutes",15,60); if(![15,30,60].includes(duration)) fail(422,"VALIDATION","Duration must be 15, 30 or 60");
    const recipients=await env.AL_DB.prepare("SELECT contact_user_id FROM contacts WHERE owner_id=? AND can_receive_alarm=1 AND blocked=0").bind(u.id).all(); if(!recipients.results.length) fail(422,"NO_TRUSTED_CONTACTS","Choose trusted contacts first");
    const eid=id(),stamp=now(),expires=new Date(Date.now()+duration*60000).toISOString(), share=bool(b.locationShared);
    await env.AL_DB.prepare("INSERT INTO emergency_events(id,initiator_id,message,location_shared,latitude,longitude,expires_at,status,created_at) VALUES(?,?,?,?,?,?,?,'active',?)").bind(eid,u.id,text(b.message,"message",1,1000),share?1:0,share?Number(b.latitude):null,share?Number(b.longitude):null,expires,stamp).run();
    if(env.AL_QUEUE) await env.AL_QUEUE.send({type:"emergency",eventId:eid,recipients:recipients.results.map(x=>x.contact_user_id)});
    await audit(env,u.id,"emergency.trigger","emergency",eid,{recipientCount:recipients.results.length}); return json(env,request,{id:eid,status:"active",expiresAt:expires,recipientCount:recipients.results.length},201);
  }
  const e=await env.AL_DB.prepare("SELECT * FROM emergency_events WHERE id=? AND initiator_id=?").bind(match[1],u.id).first(); if(!e) fail(404,"NOT_FOUND","Alarm not found");
  if(request.method==="DELETE") { await env.AL_DB.prepare("UPDATE emergency_events SET status='cancelled',cancelled_at=?,location_shared=0,latitude=NULL,longitude=NULL,expires_at=? WHERE id=?").bind(now(),now(),e.id).run(); await audit(env,u.id,"emergency.cancel","emergency",e.id); return empty(env,request); }
  fail(405,"METHOD_NOT_ALLOWED","Method not allowed");
}
export async function providers(request,env,match) {
  const u=await auth(env,request);
  if(!match&&request.method==="GET") { const r=await env.AL_DB.prepare("SELECT id,name,type,base_url,model,enabled,is_default,priority,usage_count,notes,last_used_at,processing_location FROM provider_configs WHERE user_id=? ORDER BY priority").bind(u.id).all(); return json(env,request,{providers:r.results}); }
  if(!match&&request.method==="POST") { const b=await body(request); if("apiKey" in b) fail(422,"API_KEY_FORBIDDEN","API keys must remain in encrypted local storage"); const pid=id(),stamp=now(); await env.AL_DB.prepare("INSERT INTO provider_configs(id,user_id,name,type,base_url,model,enabled,is_default,priority,usage_count,notes,processing_location,created_at) VALUES(?,?,?,?,?,?,?,?,?,0,?,?,?)").bind(pid,u.id,text(b.name,"name",1,100),oneOf(b.type,"type",["llm","music","translate","stt","tts"]),optionalText(b.baseUrl,"baseUrl",500),optionalText(b.model,"model",200),bool(b.enabled,true),bool(b.isDefault),int(b.priority,"priority",1,100,10),optionalText(b.notes,"notes",1000),oneOf(b.processingLocation,"processingLocation",["local","cloud"],"cloud"),stamp).run(); return json(env,request,{id:pid},201); }
  const owned=await env.AL_DB.prepare("SELECT * FROM provider_configs WHERE id=? AND user_id=?").bind(match[1],u.id).first(); if(!owned) fail(404,"NOT_FOUND","Provider not found");
  if(request.method==="DELETE") { await env.AL_DB.prepare("DELETE FROM provider_configs WHERE id=?").bind(owned.id).run(); return empty(env,request); }
  if(request.method==="PATCH") { const b=await body(request); if("apiKey" in b) fail(422,"API_KEY_FORBIDDEN","API keys must remain local"); await env.AL_DB.prepare("UPDATE provider_configs SET enabled=?,is_default=?,priority=?,notes=?,usage_count=?,last_used_at=? WHERE id=?").bind(b.enabled==null?owned.enabled:bool(b.enabled),b.isDefault==null?owned.is_default:bool(b.isDefault),int(b.priority,"priority",1,100,owned.priority),b.notes===undefined?owned.notes:optionalText(b.notes,"notes",1000),int(b.usageCount,"usageCount",0,1e9,owned.usage_count),b.lastUsedAt===undefined?owned.last_used_at:optionalText(b.lastUsedAt,"lastUsedAt",40),owned.id).run(); return empty(env,request); }
  fail(405,"METHOD_NOT_ALLOWED","Method not allowed");
}
