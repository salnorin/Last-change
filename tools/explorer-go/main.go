// Optional, read-only block explorer front-end in Go (stdlib only).
// It does not touch keys or the ledger: it only reads the JSON API served by
// the Rust binary (`lastchange explorer`, default http://127.0.0.1:8090) and
// renders a tiny HTML page. Not part of the cargo build.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"html/template"
	"log"
	"net/http"
	"time"
)

type block struct {
	Height int    `json:"height"`
	Hash   string `json:"hash"`
	Time   int64  `json:"time"`
	Bits   int    `json:"bits"`
}

type chain struct {
	GenesisHash string  `json:"genesis_hash"`
	TipHash     string  `json:"tip_hash"`
	Height      int     `json:"height"`
	Blocks      []block `json:"blocks"`
}

var page = template.Must(template.New("p").Parse(`<!doctype html><meta charset="utf-8">
<title>Last-change explorer (Go)</title>
<style>body{font-family:monospace;margin:2em}td{padding:2px 8px}</style>
<h1>Last-change explorer</h1><p>read-only · source {{.Src}}</p>
{{if .Err}}<p style="color:red">{{.Err}}</p>{{else}}
<p>height {{.C.Height}} · tip {{.C.TipHash}}</p>
<table><tr><th>height</th><th>hash</th><th>time (UTC)</th><th>bits</th></tr>
{{range .C.Blocks}}<tr><td>{{.Height}}</td><td>{{.Hash}}</td><td>{{.Time}}</td><td>{{.Bits}}</td></tr>{{end}}
</table>{{end}}`))

func fetch(api string) (chain, error) {
	var c chain
	cl := http.Client{Timeout: 5 * time.Second}
	r, err := cl.Get(api + "/api/blocks")
	if err != nil {
		return c, err
	}
	defer r.Body.Close()
	if r.StatusCode != 200 {
		return c, fmt.Errorf("upstream status %d", r.StatusCode)
	}
	err = json.NewDecoder(r.Body).Decode(&c)
	return c, err
}

func main() {
	api := flag.String("api", "http://127.0.0.1:8090", "Rust explorer base URL")
	addr := flag.String("http", "127.0.0.1:8091", "listen address")
	flag.Parse()
	http.HandleFunc("/", func(w http.ResponseWriter, _ *http.Request) {
		c, err := fetch(*api)
		data := map[string]any{"Src": *api, "C": c, "Err": ""}
		if err != nil {
			data["Err"] = "cannot reach " + *api + ": " + err.Error() + " (run `lastchange explorer` first)"
		}
		_ = page.Execute(w, data)
	})
	http.HandleFunc("/api/blocks", func(w http.ResponseWriter, _ *http.Request) {
		c, err := fetch(*api)
		if err != nil {
			http.Error(w, err.Error(), http.StatusBadGateway)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(c)
	})
	log.Printf("explorer-go http://%s (reading %s)", *addr, *api)
	log.Fatal(http.ListenAndServe(*addr, nil))
}
