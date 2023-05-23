package main

import "C"

import (
	"context"
	"fmt"
	"net"
	"os"
	"strconv"
	"sync"
	"time"

	"github.com/filecoin-project/lassie/pkg/lassie"
	httpserver "github.com/filecoin-project/lassie/pkg/server/http"
)

var mtx sync.Mutex
var globalCtx context.Context
var daemon *httpserver.HttpServer
var debug_log_enabled bool

// InitDaemon initializes Lassie HTTP daemon listening on localhost and returns the port number.
// The daemon is a singleton - there can be only one instance running in the host process.
//
// **Important:** This function does not run the request handler, you must call RunDaemon().
//
//export InitDaemon
func InitDaemon(debug_log bool) uint16 {
	// We cannot use debug() here because the global debug_log variable was not initialized yet
	if debug_log {
		print_debug("InitDaemon locking the mutex")
	}

	mtx.Lock()
	defer mtx.Unlock()
	debug_log_enabled = debug_log
	defer debug("InitDaemon lock released")

	if globalCtx != nil {
		// FIXME - handle errors
		panic("cannot create more than one Lassie daemon")
	}

	globalCtx = context.Background()

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

	// FIXME: configure tempDir
	// lassieOpts = append(lassieOpts, lassie.WithTempDir(tempDir))

	lassie, err := lassie.NewLassie(globalCtx, lassieOpts...)
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("cannot create Lassie instance: %s", err))
	}

	daemon, err = httpserver.NewHttpServer(globalCtx, lassie, httpserver.HttpServerConfig{
		Address: "127.0.0.1",
		// FIXME: make this configurable
		Port: 0,
		// FIXME: make this configurable
		TempDir: "",
		// No limit.
		// TODO: I think we should expose this as a config option
		MaxBlocksPerRequest: 0,
	})

	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("cannot start the HTTP server: %s", err))
	}

	return getPort()
}

// Run the daemon (the HTTP request handler). You should call this function from a dedicated
// OS-level thread.
//
// **Important:** This function does not exit until you call StopDaemon from a different thread.
//
//export RunDaemon
func RunDaemon() {
	server := getDaemon()

	if server == nil {
		// The server may have been cleaned by now if StopDaemon was calling quickly enough
		return
	}

	debug("RUNNING LASSIE HANDLER")
	err := server.Start()
	debug("LASSIE HANDLER EXITED:", err)
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("Lassie HTTP server error: %s", err))
	}
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
func StopDaemon() {
	debug("StopDaemon locking the mutex")
	mtx.Lock()
	defer mtx.Unlock()
	defer debug("StopDaemon lock released")

	debug("STOPPING LASSIE HANDLER")
	err := daemon.Close()
	debug("STOP ERROR?", err)
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("Cannot stop Lassie HTTP server: %s", err))
	}

	globalCtx = nil
	daemon = nil
}

func getPort() uint16 {
	_, portStr, err := net.SplitHostPort(daemon.Addr())
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("cannot parse server address `%s`: %s", daemon.Addr(), err))
	}
	port, err := strconv.ParseUint(portStr, 10, 16)
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("invalid port number `%s`: %s", portStr, err))
	}

	return uint16(port)
}

func debug(a ...any) {
	if debug_log_enabled {
		print_debug(a...)
	}
}

func print_debug(a ...any) {
	fmt.Fprintf(os.Stderr, "[LASSIE GO WRAPPER] ")
	fmt.Fprintln(os.Stderr, a...)
}

func main() {}
