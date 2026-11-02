import { auth, audit, b64, body, bool, digest, empty, fail, hashPassword, id, int, json, now, oneOf, optionalText, publicUser, rate, text, token, verifyPassword } from "./core.js";

const personas = ["general","mute","blind","lowVision","deaf","hardOfHearing","deafMute","deafBlind","motor","neurodivergent","caregiver","teacher","therapist","organization"];
async function issue(env, userId, deviceId) {
  const access = b64(crypto.getRandomValues(new Uint8Array(32)));
  const refresh = b64(crypto.getRandomValues(new Uint8Array(48)));
  const stamp = now(), sessionId = id();
  await env.AL_DB.prepare("INSERT INTO sessions(id,user_id,device_id,access_hash,refresh_hash,access_expires_at,refresh_expires_at,created_at,last_used_at) VALUES(?,?,?,?,?,?,?,?,?)").bind(sessionId,userId,deviceId,await digest(access),await digest(refresh),new Date(Date.now()+900000).toISOString(),new Date(Date.now()+2592000000).toISOString(),stamp,stamp).run();
  return { accessToken: access, refreshToken: refresh, accessExpiresIn: 900, refreshExpiresIn: 2592000, sessionId };
}
export async function register(request, env) {
  await rate(env, request.headers.get("cf-connecting-ip") || "unknown", "register", 5, 3600);
  const b = await body(request), email = text(b.email,"email",5,254).toLowerCase();
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) fail(422,"INVALID_EMAIL","Valid email required");
  if (typeof b.password !== "string" || b.password.length < 12 || b.password.length > 128 || !/[A-Za-z]/.test(b.password) || !/\d/.test(b.password)) fail(422,"WEAK_PASSWORD","Password must have 12–128 characters, letters and numbers");
  const displayName=text(b.displayName,"displayName",2,80), deviceId=text(request.headers.get("x-device-id"),"device",8,128);
  const publicKey=text(b.publicKey,"publicKey",16,4096), persona=oneOf(b.persona,"persona",personas,"general");
  const stamp=now(), userId=id(), publicId=`AX-${crypto.randomUUID().slice(0,8).toUpperCase()}`;
  const isAdmin=env.ADMIN_EMAIL && email===env.ADMIN_EMAIL.toLowerCase() ? 1:0;
  const recovery=b64(crypto.getRandomValues(new Uint8Array(24)));
  try {
    await env.AL_DB.batch([
      env.AL_DB.prepare("INSERT INTO users(id,public_id,email,password_hash,display_name,public_key,persona,preferred_lang,is_admin,bio,created_at,profile_completed) VALUES(?,?,?,?,?,?,?,?,?,?,?,1)").bind(userId,publicId,email,await hashPassword(b.password),displayName,publicKey,persona,oneOf(b.language,"language",["pl","en"],"pl"),isAdmin,optionalText(b.bio,"bio",1000),stamp),
      env.AL_DB.prepare("INSERT INTO accessibility_profiles(user_id,settings,updated_at) VALUES(?,?,?)").bind(userId,JSON.stringify({confirmBeforeSend:true,autoSendChannels:[],localAIPreferred:true,textScale:1,morseWpm:15,vibrationStrength:.7,ttsRate:1,ttsPitch:1,largeTargets:true}),stamp),
      env.AL_DB.prepare("INSERT INTO recovery_codes(id,user_id,code_hash,created_at) VALUES(?,?,?,?)").bind(id(),userId,await digest(recovery),stamp)
    ]);
  } catch(e) { if (String(e).includes("UNIQUE")) fail(409,"ACCOUNT_EXISTS","Account already exists"); throw e; }
  const tokens=await issue(env,userId,deviceId);
  await audit(env,userId,"account.register","user",userId);
  const user=await env.AL_DB.prepare("SELECT * FROM users WHERE id=?").bind(userId).first();
  return json(env,request,{user:publicUser(user),...tokens,recoveryCode:recovery},201);
}
export async function login(request,env) {
  const b=await body(request), email=text(b.email,"email",5,254).toLowerCase(), deviceId=text(request.headers.get("x-device-id"),"device",8,128);
  await rate(env,`${email}:${request.headers.get("cf-connecting-ip")||"unknown"}`,"login",10,300);
  const user=await env.AL_DB.prepare("SELECT * FROM users WHERE email=? AND deleted_at IS NULL").bind(email).first();
  if (!user || typeof b.password!=="string" || !await verifyPassword(b.password,user.password_hash)) fail(401,"INVALID_CREDENTIALS","Invalid email or password");
  if (user.locked_until && user.locked_until>now()) fail(423,"ACCOUNT_LOCKED","Account temporarily locked");
  const tokens=await issue(env,user.id,deviceId);
  await env.AL_DB.prepare("UPDATE users SET failed_logins=0,last_seen=? WHERE id=?").bind(now(),user.id).run();
  await audit(env,user.id,"account.login","session",tokens.sessionId);
  return json(env,request,{user:publicUser(user),...tokens});
}
export async function refresh(request,env) {
  const b=await body(request), refreshToken=text(b.refreshToken,"refreshToken",32,256), deviceId=text(request.headers.get("x-device-id"),"device",8,128), hash=await digest(refreshToken);
  const s=await env.AL_DB.prepare("SELECT * FROM sessions WHERE refresh_hash=? AND device_id=? AND revoked_at IS NULL AND refresh_expires_at>?").bind(hash,deviceId,now()).first();
  if(!s) fail(401,"INVALID_REFRESH","Refresh token is invalid or expired");
  await env.AL_DB.prepare("UPDATE sessions SET revoked_at=?,replaced_at=? WHERE id=? AND revoked_at IS NULL").bind(now(),now(),s.id).run();
  return json(env,request,await issue(env,s.user_id,deviceId));
}
export async function logout(request,env) { const u=await auth(env,request); await env.AL_DB.prepare("UPDATE sessions SET revoked_at=? WHERE id=?").bind(now(),u.sessionId).run(); await audit(env,u.id,"account.logout","session",u.sessionId); return empty(env,request); }
export async function me(request,env) { const u=await auth(env,request); const row=await env.AL_DB.prepare("SELECT * FROM users WHERE id=?").bind(u.id).first(); return json(env,request,{user:publicUser(row)}); }
export async function updateMe(request,env) {
  const u=await auth(env,request), b=await body(request);
  const row=await env.AL_DB.prepare("SELECT * FROM users WHERE id=?").bind(u.id).first();
  await env.AL_DB.prepare("UPDATE users SET display_name=?,bio=?,avatar_url=?,persona=?,preferred_lang=?,profile_completed=1 WHERE id=?").bind(b.displayName==null?row.display_name:text(b.displayName,"displayName",2,80),b.bio===undefined?row.bio:optionalText(b.bio,"bio",1000),b.avatarUrl===undefined?row.avatar_url:optionalText(b.avatarUrl,"avatarUrl",500),b.persona==null?row.persona:oneOf(b.persona,"persona",personas),b.preferredLang==null?row.preferred_lang:oneOf(b.preferredLang,"preferredLang",["pl","en"]),u.id).run();
  await audit(env,u.id,"profile.update","user",u.id); return me(request,env);
}
export async function accessibility(request,env) {
  const u=await auth(env,request);
  if(request.method==="GET") { const r=await env.AL_DB.prepare("SELECT settings,updated_at FROM accessibility_profiles WHERE user_id=?").bind(u.id).first(); return json(env,request,{profile:JSON.parse(r?.settings||"{}"),updatedAt:r?.updated_at}); }
  const b=await body(request,32768); if(b.autoSendChannels && (!Array.isArray(b.autoSendChannels)||b.autoSendChannels.some(x=>!["text","speech","morse","sign","image"].includes(x)))) fail(422,"VALIDATION","Invalid autoSendChannels");
  await env.AL_DB.prepare("INSERT INTO accessibility_profiles(user_id,settings,updated_at) VALUES(?,?,?) ON CONFLICT(user_id) DO UPDATE SET settings=excluded.settings,updated_at=excluded.updated_at").bind(u.id,JSON.stringify(b),now()).run();
  await audit(env,u.id,"accessibility.update","user",u.id); return json(env,request,{profile:b});
}
export async function contacts(request,env,match) {
  const u=await auth(env,request);
  if(request.method==="GET") { const rows=await env.AL_DB.prepare("SELECT c.*,u.public_id,u.display_name,u.avatar_url,u.bio,u.public_key,u.persona,u.preferred_lang,u.is_verified,u.created_at,u.last_seen FROM contacts c JOIN users u ON u.id=c.contact_user_id WHERE c.owner_id=? AND c.blocked=0 ORDER BY c.display_name").bind(u.id).all(); return json(env,request,{contacts:rows.results.map(r=>({id:r.id,verified:!!r.verified,safetyNumber:r.safety_number,canReceiveAlarm:!!r.can_receive_alarm,addedAt:r.added_at,user:publicUser(r)}))}); }
  if(request.method==="POST") {
    const b=await body(request), publicId=text(b.publicId,"publicId",4,64).toUpperCase(), contact=await env.AL_DB.prepare("SELECT * FROM users WHERE public_id=? AND deleted_at IS NULL").bind(publicId).first();
    if(!contact) fail(404,"NOT_FOUND","User not found"); if(contact.id===u.id) fail(422,"VALIDATION","Cannot add yourself");
    await env.AL_DB.prepare("INSERT INTO contacts(id,owner_id,contact_user_id,display_name,added_at) VALUES(?,?,?,?,?) ON CONFLICT(owner_id,contact_user_id) DO UPDATE SET display_name=excluded.display_name").bind(id(),u.id,contact.id,optionalText(b.displayName,"displayName",80)||contact.display_name,now()).run();
    await audit(env,u.id,"contact.add","contact",contact.id); return json(env,request,{contact:publicUser(contact)},201);
  }
  const contactId=match[1];
  if(request.method==="DELETE") { await env.AL_DB.prepare("DELETE FROM contacts WHERE owner_id=? AND contact_user_id=?").bind(u.id,contactId).run(); return empty(env,request); }
  const b=await body(request), existing=await env.AL_DB.prepare("SELECT * FROM contacts WHERE owner_id=? AND contact_user_id=?").bind(u.id,contactId).first(); if(!existing) fail(404,"NOT_FOUND","Contact not found");
  await env.AL_DB.prepare("UPDATE contacts SET verified=?,safety_number=?,can_receive_alarm=?,blocked=? WHERE owner_id=? AND contact_user_id=?").bind(b.verified==null?existing.verified:bool(b.verified),b.safetyNumber===undefined?existing.safety_number:optionalText(b.safetyNumber,"safetyNumber",256),b.canReceiveAlarm==null?existing.can_receive_alarm:bool(b.canReceiveAlarm),b.blocked==null?existing.blocked:bool(b.blocked),u.id,contactId).run();
  await audit(env,u.id,"contact.update","contact",contactId); return empty(env,request);
}
export async function exportData(request,env) {
  const u=await auth(env,request);
  const [user,msgs,posts,consents]=await Promise.all([env.AL_DB.prepare("SELECT id,public_id,email,display_name,avatar_url,bio,public_key,persona,preferred_lang,created_at FROM users WHERE id=?").bind(u.id).first(),env.AL_DB.prepare("SELECT * FROM messages WHERE sender_id=?").bind(u.id).all(),env.AL_DB.prepare("SELECT * FROM community_posts WHERE author_id=?").bind(u.id).all(),env.AL_DB.prepare("SELECT * FROM consent_records WHERE user_id=?").bind(u.id).all()]);
  await audit(env,u.id,"privacy.export","user",u.id); return json(env,request,{format:"axbridge-export-v1",exportedAt:now(),user,messages:msgs.results,communityPosts:posts.results,consents:consents.results});
}
export async function deleteAccount(request,env) {
  const u=await auth(env,request), b=await body(request); if(b.confirmation!=="USUŃ KONTO") fail(422,"CONFIRMATION_REQUIRED","Type USUŃ KONTO to confirm");
  await env.AL_DB.prepare("UPDATE users SET deleted_at=?,email=NULL,display_name='Usunięte konto',avatar_url=NULL,bio=NULL,public_key='revoked' WHERE id=?").bind(now(),u.id).run();
  await env.AL_DB.prepare("UPDATE sessions SET revoked_at=? WHERE user_id=?").bind(now(),u.id).run(); await audit(env,u.id,"privacy.erase","user",u.id); return empty(env,request);
}
