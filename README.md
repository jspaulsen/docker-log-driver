# Docker Log Driver

Docker Log Driver plugin.  For use with [Log Ingest Api](https://github.com/jspaulsen/log-ingest-api).
## Plugin Configuration

Done via docker daemon script

## Plugin installation
```bash
make image-build
make create-plugin
```

## Build 
### Dependencies

#### docker_protobuf

* `protoc` - Protocol Buffers compiler; ```sudo apt-get install protobuf-compiler```
* Make and related
