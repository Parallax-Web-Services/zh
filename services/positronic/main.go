// Positronic control plane scaffold.
// Intentionally not an orchestrator yet; only a local health endpoint is exposed.
package main

import (
	"encoding/json"
	"log"
	"net"
	"net/http"
	"os"
	"time"
)

type reply struct {
	Service string `json:"service"`
	Version string `json:"version"`
	State   string `json:"state"`
}

func handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Cache-Control", "no-store")
		_ = json.NewEncoder(w).Encode(reply{
			Service: "positronic", Version: "0.1.0-alpha", State: "scaffold_only",
		})
	})
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "control-plane APIs are not implemented", http.StatusNotImplemented)
	})
	return mux
}

func main() {
	// Only loopback until node identity, auth, trust, and authorization exist.
	address := "127.0.0.1:9460"
	if os.Getenv("POSITRONIC_ADDR") != "" {
		log.Fatal("POSITRONIC_ADDR override is disabled until authentication is implemented")
	}
	listener, err := net.Listen("tcp", address)
	if err != nil {
		log.Fatal(err)
	}
	server := &http.Server{
		Handler: handler(),
		ReadHeaderTimeout: 3 * time.Second,
	}
	log.Printf("Positronic scaffold (no scheduling) listening on http://%s/healthz", address)
	log.Fatal(server.Serve(listener))
}
