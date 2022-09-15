package goatcrouter

import (
	"testing"

	"github.com/gofrs/uuid"
	"github.com/stretchr/testify/require"
)

func verify(atc string) error {
	schema := NewSchema()
	defer schema.Free()

	schema.AddField("http.path", String)
	schema.AddField("tcp.port", Int)

	router := NewRouter(schema)
	defer router.Free()

	id, err := uuid.NewV4()
	if err != nil {
		return err
	}

	return router.AddMatcher(1, id, atc)
}

func Test_Verify(t *testing.T) {
	require.NoError(t, verify("tcp.port == 1"))
	require.Error(t, verify("bad.var == 9"))
}
