begin;

create or replace function public.increment_numeric_string(value text)
returns text
language plpgsql
immutable
strict
set search_path = public, pg_temp
as $$
declare
  digits text[];
  idx integer;
  carry integer := 1;
  current_digit integer;
begin
  if value = '' or value !~ '^[0-9]+$' then
    return null;
  end if;

  digits := string_to_array(value, null);

  idx := array_length(digits, 1);
  while idx >= 1 and carry > 0 loop
    current_digit := digits[idx]::integer + carry;
    if current_digit >= 10 then
      digits[idx] := '0';
      carry := 1;
    else
      digits[idx] := current_digit::text;
      carry := 0;
    end if;
    idx := idx - 1;
  end loop;

  if carry > 0 then
    digits := array_prepend('1', digits);
  end if;

  return array_to_string(digits, '');
end;
$$;

create or replace function public.normalize_memo_image_path(raw text)
returns text
language plpgsql
immutable
set search_path = public, pg_temp
as $$
declare
  trimmed text;
  cleaned text;
  parsed text;
begin
  if raw is null then
    return null;
  end if;

  trimmed := btrim(raw);
  if trimmed = '' then
    return null;
  end if;

  if trimmed ~* '^(data:|blob:)' then
    return null;
  end if;

  if position('://' in trimmed) = 0 then
    cleaned := regexp_replace(trimmed, '^/+', '');

    if cleaned like 'memo-images/%' then
      cleaned := substr(cleaned, length('memo-images/') + 1);
    elsif cleaned like 'public/memo-images/%' then
      cleaned := substr(cleaned, length('public/memo-images/') + 1);
    elsif cleaned like 'sign/memo-images/%' then
      cleaned := substr(cleaned, length('sign/memo-images/') + 1);
    end if;

    if cleaned = '' then
      return null;
    end if;

    return cleaned;
  end if;

  parsed := substring(trimmed from '/storage/v1/object/public/memo-images/([^?]+)');
  if parsed is null or parsed = '' then
    parsed := substring(trimmed from '/storage/v1/object/sign/memo-images/([^?]+)');
  end if;
  if parsed is null or parsed = '' then
    parsed := substring(trimmed from '/storage/v1/object/memo-images/([^?]+)');
  end if;

  if parsed is null or parsed = '' then
    return null;
  end if;

  return parsed;
end;
$$;

create or replace function public.memo_update_resolve_conflict(
  arg_memo_id uuid,
  arg_text text,
  arg_expected_version text,
  arg_image_urls text[] default null
)
returns table (
  status text,
  memo_id uuid,
  memo jsonb,
  server_memo jsonb,
  forked_memo jsonb
)
language plpgsql
security invoker
set search_path = public, pg_temp
as $$
declare
  v_actor_id uuid;
  v_existing public.memos%rowtype;
  v_updated public.memos%rowtype;
  v_forked public.memos%rowtype;
begin
  v_actor_id := auth.uid();
  if v_actor_id is null then
    return query
    select
      'not_found'::text,
      arg_memo_id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  select *
  into v_existing
  from public.memos
  where id = arg_memo_id
    and user_id = v_actor_id
  for update;

  if not found then
    return query
    select
      'not_found'::text,
      arg_memo_id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  if v_existing.version = arg_expected_version then
    update public.memos
    set
      text = arg_text,
      updated_at = clock_timestamp(),
      version = coalesce(public.increment_numeric_string(version), version)
    where id = v_existing.id
    returning * into v_updated;

    if arg_image_urls is not null then
      delete from public.memo_images
      where memo_id = v_updated.id;

      insert into public.memo_images (memo_id, url, sort_order)
      select
        v_updated.id,
        normalized_urls.normalized_url,
        normalized_urls.sort_order
      from (
        select
          public.normalize_memo_image_path(raw_url) as normalized_url,
          (ordinality - 1)::integer as sort_order
        from unnest(arg_image_urls) with ordinality as input_urls(raw_url, ordinality)
      ) as normalized_urls
      where normalized_urls.normalized_url is not null
        and normalized_urls.normalized_url <> '';
    end if;

    return query
    select
      'updated'::text,
      v_updated.id,
      (
        select
          to_jsonb(m)
          || jsonb_build_object(
            'memo_images',
            coalesce(
              (
                select jsonb_agg(
                  jsonb_build_object('url', mi.url, 'sort_order', mi.sort_order)
                  order by mi.sort_order
                )
                from public.memo_images mi
                where mi.memo_id = m.id
              ),
              '[]'::jsonb
            )
          )
        from public.memos m
        where m.id = v_updated.id
      ),
      null::jsonb,
      null::jsonb;
    return;
  end if;

  insert into public.memos (user_id, text)
  values (v_actor_id, arg_text)
  returning * into v_forked;

  insert into public.memo_images (memo_id, url, sort_order)
  select
    v_forked.id,
    mi.url,
    mi.sort_order
  from public.memo_images mi
  where mi.memo_id = v_existing.id
  order by mi.sort_order;

  return query
  select
    'conflict'::text,
    v_existing.id,
    null::jsonb,
    (
      select
        to_jsonb(m)
        || jsonb_build_object(
          'memo_images',
          coalesce(
            (
              select jsonb_agg(
                jsonb_build_object('url', mi.url, 'sort_order', mi.sort_order)
                order by mi.sort_order
              )
              from public.memo_images mi
              where mi.memo_id = m.id
            ),
            '[]'::jsonb
          )
        )
      from public.memos m
      where m.id = v_existing.id
    ),
    (
      select
        to_jsonb(m)
        || jsonb_build_object(
          'memo_images',
          coalesce(
            (
              select jsonb_agg(
                jsonb_build_object('url', mi.url, 'sort_order', mi.sort_order)
                order by mi.sort_order
              )
              from public.memo_images mi
              where mi.memo_id = m.id
            ),
            '[]'::jsonb
          )
        )
      from public.memos m
      where m.id = v_forked.id
    );
end;
$$;

create or replace function public.memo_delete_resolve_conflict(
  arg_memo_id uuid,
  arg_expected_version text,
  arg_deleted_at timestamptz default null
)
returns table (
  status text,
  memo_id uuid,
  memo jsonb,
  server_memo jsonb,
  forked_memo jsonb
)
language plpgsql
security invoker
set search_path = public, pg_temp
as $$
declare
  v_actor_id uuid;
  v_existing public.memos%rowtype;
  v_deleted_at timestamptz;
begin
  v_actor_id := auth.uid();
  if v_actor_id is null then
    return query
    select
      'not_found'::text,
      arg_memo_id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  select *
  into v_existing
  from public.memos
  where id = arg_memo_id
    and user_id = v_actor_id
  for update;

  if not found then
    return query
    select
      'not_found'::text,
      arg_memo_id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  if v_existing.version = arg_expected_version then
    v_deleted_at := coalesce(arg_deleted_at, clock_timestamp());

    update public.memos
    set
      deleted_at = v_deleted_at,
      updated_at = v_deleted_at,
      version = coalesce(public.increment_numeric_string(version), version)
    where id = v_existing.id;

    return query
    select
      'deleted'::text,
      v_existing.id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  return query
  select
    'conflict'::text,
    v_existing.id,
    null::jsonb,
    (
      select
        to_jsonb(m)
        || jsonb_build_object(
          'memo_images',
          coalesce(
            (
              select jsonb_agg(
                jsonb_build_object('url', mi.url, 'sort_order', mi.sort_order)
                order by mi.sort_order
              )
              from public.memo_images mi
              where mi.memo_id = m.id
            ),
            '[]'::jsonb
          )
        )
      from public.memos m
      where m.id = v_existing.id
    ),
    null::jsonb;
end;
$$;

create or replace function public.memo_restore_resolve_conflict(
  arg_memo_id uuid,
  arg_expected_version text,
  arg_restored_at timestamptz default null
)
returns table (
  status text,
  memo_id uuid,
  memo jsonb,
  server_memo jsonb,
  forked_memo jsonb
)
language plpgsql
security invoker
set search_path = public, pg_temp
as $$
declare
  v_actor_id uuid;
  v_existing public.memos%rowtype;
  v_restored_at timestamptz;
  v_restored public.memos%rowtype;
begin
  v_actor_id := auth.uid();
  if v_actor_id is null then
    return query
    select
      'not_found'::text,
      arg_memo_id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  select *
  into v_existing
  from public.memos
  where id = arg_memo_id
    and user_id = v_actor_id
  for update;

  if not found then
    return query
    select
      'not_found'::text,
      arg_memo_id,
      null::jsonb,
      null::jsonb,
      null::jsonb;
    return;
  end if;

  if v_existing.version = arg_expected_version then
    v_restored_at := coalesce(arg_restored_at, clock_timestamp());

    update public.memos
    set
      deleted_at = null,
      updated_at = v_restored_at,
      version = coalesce(public.increment_numeric_string(version), version)
    where id = v_existing.id
    returning * into v_restored;

    return query
    select
      'restored'::text,
      v_restored.id,
      (
        select
          to_jsonb(m)
          || jsonb_build_object(
            'memo_images',
            coalesce(
              (
                select jsonb_agg(
                  jsonb_build_object('url', mi.url, 'sort_order', mi.sort_order)
                  order by mi.sort_order
                )
                from public.memo_images mi
                where mi.memo_id = m.id
              ),
              '[]'::jsonb
            )
          )
        from public.memos m
        where m.id = v_restored.id
      ),
      null::jsonb,
      null::jsonb;
    return;
  end if;

  return query
  select
    'conflict'::text,
    v_existing.id,
    null::jsonb,
    (
      select
        to_jsonb(m)
        || jsonb_build_object(
          'memo_images',
          coalesce(
            (
              select jsonb_agg(
                jsonb_build_object('url', mi.url, 'sort_order', mi.sort_order)
                order by mi.sort_order
              )
              from public.memo_images mi
              where mi.memo_id = m.id
            ),
            '[]'::jsonb
          )
        )
      from public.memos m
      where m.id = v_existing.id
    ),
    null::jsonb;
end;
$$;

revoke all on function public.memo_update_resolve_conflict(uuid, text, text, text[]) from public;
revoke all on function public.memo_delete_resolve_conflict(uuid, text, timestamptz) from public;
revoke all on function public.memo_restore_resolve_conflict(uuid, text, timestamptz) from public;

revoke execute on function public.memo_update_resolve_conflict(uuid, text, text, text[]) from anon;
revoke execute on function public.memo_delete_resolve_conflict(uuid, text, timestamptz) from anon;
revoke execute on function public.memo_restore_resolve_conflict(uuid, text, timestamptz) from anon;

grant execute on function public.memo_update_resolve_conflict(uuid, text, text, text[]) to authenticated;
grant execute on function public.memo_delete_resolve_conflict(uuid, text, timestamptz) to authenticated;
grant execute on function public.memo_restore_resolve_conflict(uuid, text, timestamptz) to authenticated;

commit;
