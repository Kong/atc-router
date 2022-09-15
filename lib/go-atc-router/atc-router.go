package goatcrouter

// #cgo LDFLAGS: -L${SRCDIR}/../../target/release -Wl,-Bstatic -latc_router -Wl,-Bdynamic
// #include "atc-router.h"
import "C"

import (
	"fmt"
	"runtime"
	"unsafe"

	"github.com/gofrs/uuid"
)

// FieldType serves to indicate the desired data type for a Schema field.
type FieldType int

const (
	String FieldType = C.Type_String
	IpCidr FieldType = C.Type_IpCidr
	IpAddr FieldType = C.Type_IpAddr
	Int    FieldType = C.Type_Int
	Regex  FieldType = C.Type_Regex
)

// The Schema type holds the names and types of fields available to the router.
type Schema struct {
	s *C.Schema
}

// NewSchema creates a new empty Schema object
func NewSchema() *Schema {
	s := &Schema{s: C.schema_new()}
	runtime.SetFinalizer(s, (*Schema).Free)
	return s
}

// The Free method deallocates a Schema object
// can be called manually or automatically by the GC.
func (s *Schema) Free() {
	runtime.SetFinalizer(s, nil)
	C.schema_free(s.s)
}

// AddField is used to define fields and their associated type.
func (s *Schema) AddField(field string, typ FieldType) {
	fieldC := C.CString(field)
	defer C.free(unsafe.Pointer(fieldC))

	C.schema_add_field(s.s, (*C.schar)(fieldC), uint32(typ))
}

// The Router type holds the Matcher rules.
type Router struct {
	r *C.Router
}

// NewRouter creates a new empty Router object associated with
// the given Schema.
func NewRouter(s *Schema) *Router {
	if s == nil {
		return nil
	}

	r := &Router{r: C.router_new(s.s)}
	runtime.SetFinalizer(r, (*Router).Free)
	return r
}

// The Free method deallocates a Router object
// can be called manually or automatically by the GC.
func (r *Router) Free() {
	runtime.SetFinalizer(r, nil)
	C.router_free(r.r)
}

// AddMatcher parses a new ATC rule and adds to the Router
// under the given priority and ID.
func (r *Router) AddMatcher(priority int, id uuid.UUID, atc string) error {
	idC := C.CString(id.String())
	defer C.free(unsafe.Pointer(idC))

	errLen := C.ulong(1024)
	errBuf := [1024]C.uchar{}
	atcC := C.CString(atc)
	defer C.free(unsafe.Pointer(atcC))

	ok := C.router_add_matcher(r.r, C.ulong(priority), (*C.schar)(idC), (*C.schar)(atcC), &errBuf[0], &errLen)
	if !ok {
		return fmt.Errorf(string(errBuf[:errLen]))
	}
	return nil
}
