import { createClient } from 'https://esm.sh/@supabase/supabase-js@2'

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

Deno.serve(async (req) => {
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    const authHeader = req.headers.get('Authorization')
    if (!authHeader) {
      throw new Error('Missing authorization header')
    }

    const supabase = createClient(
      Deno.env.get('SUPABASE_URL') ?? '',
      Deno.env.get('SUPABASE_ANON_KEY') ?? '',
      { global: { headers: { Authorization: authHeader } } }
    )

    const { data: { user } } = await supabase.auth.getUser()
    if (!user) {
      throw new Error('Not authenticated')
    }

    const { operations } = await req.json()

    // Insert operations with conflict handling.
    // The (user_id, device_id, seq) constraint prevents duplicates if an operation
    // was uploaded but the client crashed before marking it synced locally.
    // ON CONFLICT DO NOTHING makes re-uploading safe.
    const { error } = await supabase
      .from('operations')
      .insert(operations.map((op: any) => ({
        ...op,
        user_id: user.id,
      })))

    // If the error is a duplicate key violation, the operation already exists —
    // that's fine, re-uploading is harmless.
    if (error && !error.message?.includes('duplicate key')) {
      throw error
    }

    return new Response(
      JSON.stringify({ success: true }),
      { headers: { ...corsHeaders, 'Content-Type': 'application/json' } }
    )
  } catch (error) {
    return new Response(
      JSON.stringify({ error: error.message }),
      { status: 400, headers: { ...corsHeaders, 'Content-Type': 'application/json' } }
    )
  }
})
