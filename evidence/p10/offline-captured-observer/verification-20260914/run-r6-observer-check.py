import hashlib,json,os,pathlib,resource,struct,subprocess,time
p=pathlib.Path(__file__).parent
out=p/"r6-observer-execution"
out.mkdir()
b=pathlib.Path("/tmp/nsbu-opencode-offline-observer-target/release/p10-offline-captured-observer")
manifest=p/"captured-metadata/r6-endpoint-local-manifest.json"
record=pathlib.Path("/mnt/niva-array/nsbu-solver/work/p10-m512-endpoint-r6-artifacts-20260913/output/step-048-clock-4096/record.json")
preflight=json.loads((p/"r6-final-preflight.json").read_text())
mem=dict((line.split()[0].rstrip(":"),int(line.split()[1])*1024) for line in pathlib.Path("/proc/meminfo").read_text().splitlines() if len(line.split())>=3)
assert mem["MemAvailable"] >= preflight["ledger"]["total_bytes"]+32*1024**3
assert hashlib.sha256(b.read_bytes()).hexdigest()=="66b4e2f4105db5377a207257e272f75ed0763e49f0493d50333cfc3be6a7cb8b"
args=["timeout","--signal=TERM","--kill-after=30s","1800s","/usr/bin/time","-v","-o",str(out/"time.txt"),str(b),"run",str(manifest),"768","32",str(120*1024**3),"rustfft-6.4.1-avx-avx2-fma"]
start=time.time()
receipt={"observer_source_commit":"9adc021ae3e0d3bed3411f209f148d9f065f22ad","binary_sha256":hashlib.sha256(b.read_bytes()).hexdigest(),"manifest_sha256":hashlib.sha256(manifest.read_bytes()).hexdigest(),"command":args,"started_epoch":start,"deadline_epoch":start+1800,"mem_available_bytes":mem["MemAvailable"],"required_headroom_bytes":32*1024**3,"api_bytes":preflight["ledger"]["total_bytes"],"address_space_limit_bytes":160*1024**3,"qualification":False}
def limits():resource.setrlimit(resource.RLIMIT_AS,(160*1024**3,160*1024**3))
with (out/"output.json").open("xb") as stdout,(out/"stderr").open("xb") as stderr:
 proc=subprocess.Popen(args,stdout=stdout,stderr=stderr,start_new_session=True,preexec_fn=limits)
 receipt["pid"]=proc.pid;receipt["pgid"]=os.getpgid(proc.pid)
 (out/"launch.json").write_text(json.dumps(receipt,indent=2)+"\n")
 code=proc.wait()
result={"exit_code":code,"wall_seconds":time.time()-start,"qualification":False,"scope":"offline-versus-recorded-inline-balance-parity; not PDE acceptance"}
if code==0:
 actual=json.loads((out/"output.json").read_text());expected=json.loads(record.read_text())
 result["balance_fields"]={k:{"expected":v,"actual":actual["balance"][k],"bits_equal":struct.pack("!d",v)==struct.pack("!d",actual["balance"][k])} for k,v in expected["balance"].items()}
 result["all_11_balance_fields_bitwise_equal"]=len(result["balance_fields"])==11 and all(v["bits_equal"] for v in result["balance_fields"].values())
 result["coefficient_hash_matches_record"]=actual["snapshot"]["coefficient_sha256"]==expected["state_sha256"]
 result["file_hash_reverified"]=actual["snapshot"]["file_sha256"]==json.loads(manifest.read_text())["file_sha256"]
(out/"result.json").write_text(json.dumps(result,indent=2)+"\n")
