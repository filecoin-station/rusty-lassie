package main

import "C"

// import (
// 	"github.com/filecoin-project/lassie/pkg/lassie"
// 	httpserver "github.com/filecoin-project/lassie/pkg/server/http"
// )

// StartDaemon initializes Lassie HTTP daemon listening on localhost and returns the port number.
// The daemon is a singleton - there can be only one instance running in the host process.
//
//export StartDaemon
func StartDaemon() uint16 {
	// todo
	return 0
}

// CloseDaemon stops the Lassie HTTP daemon.
//
//export StopDaemon
func StopDaemon() {
	// todo
}

func main() {}
