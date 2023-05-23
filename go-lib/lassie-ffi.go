package main

import "C"

import (
	"context"
	"fmt"
	"net"
	"strconv"
	"sync"
	"time"

	"github.com/filecoin-project/lassie/pkg/lassie"
	httpserver "github.com/filecoin-project/lassie/pkg/server/http"
)

var mtx sync.Mutex
var globalCtx context.Context
var httpServer *httpserver.HttpServer

// StartDaemon initializes Lassie HTTP daemon listening on localhost and returns the port number.
// The daemon is a singleton - there can be only one instance running in the host process.
//
//export StartDaemon
func StartDaemon() uint16 {
	mtx.Lock()
	defer mtx.Unlock()

	if globalCtx != nil {
		// The server is already running
		return getPort()

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

	httpServer, err = httpserver.NewHttpServer(globalCtx, lassie, httpserver.HttpServerConfig{
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

	go func() {
		err := httpServer.Start()
		if err != nil {
			// FIXME - handle errors
			panic(fmt.Sprintf("Lassie HTTP server error: %s", err))
		}
	}()

	// FIXME: if we don't print to stdout, then the coroutine running the server handler
	// does not start soon enough and the Rust side cannot make HTTP requests because
	// connections are refused on the Go side
	//
	time.Sleep(1000 * time.Millisecond)
	fmt.Println("server listening on", httpServer.Addr())

	return getPort()
}

// CloseDaemon stops the Lassie HTTP daemon.
//
//export StopDaemon
func StopDaemon() {
	mtx.Lock()
	defer mtx.Unlock()

	err := httpServer.Close()
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("Cannot stop Lassie HTTP server: %s", err))
	}

	globalCtx = nil
	httpServer = nil
}

func getPort() uint16 {
	_, portStr, err := net.SplitHostPort(httpServer.Addr())
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("cannot parse server address `%s`: %s", httpServer.Addr(), err))
	}
	port, err := strconv.ParseUint(portStr, 10, 16)
	if err != nil {
		// FIXME - handle errors
		panic(fmt.Sprintf("invalid port number `%s`: %s", portStr, err))
	}

	return uint16(port)
}

func main() {}
