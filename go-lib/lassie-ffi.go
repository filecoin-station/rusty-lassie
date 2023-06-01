package main

// Log levels - matching Rust enum log::LevelFilter
//  1 error
//  2 warn
//  3 info
//  4 debug
//  5 trace

/*
#include <stdlib.h>
#include <stdint.h>

typedef struct {
	const char* temp_dir;
	uint16_t port;
	size_t log_level;
} daemon_config_t;

typedef struct {
	uint16_t port;
	const char* error;
} daemon_init_result_t;

typedef struct {
	const char * error;
} result_t;
*/
import "C"

import (
	"context"
	"fmt"
	"net"
	"os"
	"strconv"
	"sync"
	"time"
	"unsafe"

	"github.com/filecoin-project/lassie/pkg/lassie"
	httpserver "github.com/filecoin-project/lassie/pkg/server/http"
)

var mtx sync.Mutex
var daemon *httpserver.HttpServer
var debug_log_enabled bool

var OK C.result_t = C.result_t{error: nil}

// InitDaemon initializes Lassie HTTP daemon listening on localhost and returns the port number.
// The daemon is a singleton - there can be only one instance running in the host process.
//
// **Important:** This function does not run the request handler, you must call RunDaemon().
//
//export InitDaemon
func InitDaemon(cfg *C.daemon_config_t) C.daemon_init_result_t {
	// We cannot set the global debug_log_variable here, because we need to obtain the lock first.
	// We create a local variable with a different name instead.
	wants_debug_log := cfg.log_level >= 4
	// We cannot use debug() here because the global debug_log variable was not initialized yet
	if wants_debug_log {
		print_debug("InitDaemon locking the mutex")
	}

	mtx.Lock()
	defer mtx.Unlock()
	defer debug("InitDaemon lock released")

	debug_log_enabled = wants_debug_log

	if daemon != nil {
		return newInitError("cannot create more than one Lassie daemon", nil)
	}

	var tempDir string = C.GoString(cfg.temp_dir)
	if debug_log_enabled {
		tempDirStr := "`" + tempDir + "`"
		if tempDir == "" {
			tempDirStr = "<empty>"
		}
		debug(fmt.Sprintf("Lassie configuration:\n  log_level=%d\n  port=%d\n  temp_dir=`%v`", cfg.log_level, cfg.port, tempDirStr))
	}

	lassieOpts := []lassie.LassieOption{lassie.WithProviderTimeout(20 * time.Second)}
	lassieOpts = append(lassieOpts, lassie.WithGlobalTimeout(20*time.Second))

	// TODO: configure Libp2p connection manager (LowWater, HighWater)
	// TODO: configure max concurrent SP retrievals
	// connManager, err := connmgr.NewConnManager(libp2pLowWater, libp2pHighWater)
	// if err != nil {
	// 	return err
	// }
	// lassieOpts = append(
	// 	lassieOpts,
	// 	lassie.WithLibp2pOpts(libp2p.ConnectionManager(connManager)),
	// 	lassie.WithConcurrentSPRetrievals(concurrentSPRetrievals),
	// )

	// TODO: configure bitswap concurrency
	// lassieOpts = append(lassieOpts, lassie.WithBitswapConcurrency(bitswapConcurrency))

	if tempDir != "" {
		lassieOpts = append(lassieOpts, lassie.WithTempDir(tempDir))
	}

	ctx := context.Background()

	lassie, err := lassie.NewLassie(ctx, lassieOpts...)
	if err != nil {
		return newInitError("cannot create Lassie instance", err)
	}

	daemon, err = httpserver.NewHttpServer(ctx, lassie, httpserver.HttpServerConfig{
		Address: "127.0.0.1",
		Port:    uint(cfg.port),
		TempDir: tempDir,
		// No limit.
		// TODO: I think we should expose this as a config option
		MaxBlocksPerRequest: 0,
	})

	if err != nil {
		return newInitError("cannot start the HTTP server", err)
	}

	port, err := getPort()
	if err != nil {
		return newInitError("cannot parse HTTP server port", err)
	}

	return C.daemon_init_result_t{
		port:  C.ushort(port),
		error: nil,
	}
}

func newInitError(msg string, cause error) C.daemon_init_result_t {
	if cause != nil {
		msg = fmt.Sprintf("%s: %+v", msg, cause)
	}

	return C.daemon_init_result_t{
		port:  0,
		error: C.CString(msg),
	}
}

// DropDaemonInitResult cleans up any resources allocated for and owned by the passed
// daemon_init_result_t value.
//
//export DropDaemonInitResult
func DropDaemonInitResult(result *C.daemon_init_result_t) {
	if result.error != nil {
		C.free(unsafe.Pointer(result.error))
		result.error = nil
	}
}

func newError(msg string, cause error) C.result_t {
	if cause != nil {
		msg = fmt.Sprintf("%s: %+v", msg, cause)
	}
	return C.result_t{
		error: C.CString(msg),
	}
}

// DropResult cleans up any resources allocated for and owned by the result_t value.
//
//export DropResult
func DropResult(result *C.result_t) {
	if result.error != nil {
		C.free(unsafe.Pointer(result.error))
		result.error = nil
	}
}

// Run the daemon (the HTTP request handler). You should call this function from a dedicated
// OS-level thread.
//
// **Important:** This function does not exit until you call StopDaemon from a different thread.
//
//export RunDaemon
func RunDaemon() C.result_t {
	server := getDaemon()

	if server == nil {
		// The server may have been cleaned by now if StopDaemon was calling quickly enough
		return OK
	}

	debug("RUNNING LASSIE HANDLER")
	err := server.Start()
	debug("LASSIE HANDLER EXITED:", err)
	if err != nil {
		return newError("Lassie HTTP server error", err)
	}

	return OK
}

func getDaemon() *httpserver.HttpServer {
	debug("RunDaemon locking the mutex")
	mtx.Lock()
	defer mtx.Unlock()
	defer debug("RunDaemon lock released")

	return daemon
}

// CloseDaemon stops the Lassie HTTP daemon.
//
//export StopDaemon
func StopDaemon() C.result_t {
	debug("StopDaemon locking the mutex")
	mtx.Lock()
	defer mtx.Unlock()
	defer debug("StopDaemon lock released")

	if daemon == nil {
		return newError("Lassie daemon not running, cannot stop it", nil)
	}

	debug("STOPPING LASSIE HANDLER")
	err := daemon.Close()
	debug("STOP ERROR?", err)
	if err != nil {
		return newError("Cannot stop Lassie HTTP server", err)
	}

	daemon = nil
	return OK
}

func getPort() (uint16, error) {
	_, portStr, err := net.SplitHostPort(daemon.Addr())
	if err != nil {
		return 0, fmt.Errorf("malformed server address `%s`: %+v", daemon.Addr(), err)
	}
	port, err := strconv.ParseUint(portStr, 10, 16)
	if err != nil {
		return 0, fmt.Errorf("invalid port number `%s`: %+v", portStr, err)
	}

	return uint16(port), nil
}

func debug(a ...any) {
	if debug_log_enabled {
		print_debug(a...)
	}
}

func print_debug(a ...any) {
	fmt.Fprint(os.Stderr, "[LASSIE GO WRAPPER] ")
	fmt.Fprintln(os.Stderr, a...)
}

func main() {}
