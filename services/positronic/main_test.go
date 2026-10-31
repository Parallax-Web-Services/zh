package main

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestHealth(t *testing.T) {
	r := httptest.NewRequest(http.MethodGet, "/healthz", nil)
	w := httptest.NewRecorder()
	handler().ServeHTTP(w, r)
	if w.Code != http.StatusOK || !strings.Contains(w.Body.String(), "scaffold_only") {
		t.Fatalf("unexpected health response: status=%d body=%q", w.Code, w.Body.String())
	}
}

func TestNoAccidentalOrchestratorAPI(t *testing.T) {
	r := httptest.NewRequest(http.MethodPost, "/v1/dispatch", nil)
	w := httptest.NewRecorder()
	handler().ServeHTTP(w, r)
	if w.Code != http.StatusNotImplemented {
		t.Fatalf("wanted 501, got %d", w.Code)
	}
}
