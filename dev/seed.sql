CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE public.customers (
    id serial PRIMARY KEY,
    email text NOT NULL UNIQUE,
    name text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.orders (
    id serial PRIMARY KEY,
    customer_id integer NOT NULL REFERENCES public.customers (id),
    status text NOT NULL DEFAULT 'pending',
    total numeric(10, 2)
);

CREATE INDEX orders_status_idx ON public.orders (status);

CREATE VIEW public.order_totals AS
SELECT c.email, sum(o.total) AS total
FROM public.customers c
JOIN public.orders o ON o.customer_id = c.id
GROUP BY c.email;

CREATE SCHEMA analytics;

CREATE TABLE analytics.events (
    id bigserial PRIMARY KEY,
    name text NOT NULL,
    payload jsonb
);

INSERT INTO public.customers (email, name)
SELECT 'customer' || n || '@example.com', 'Customer ' || n
FROM generate_series(1, 250) AS n;

INSERT INTO public.orders (customer_id, status, total)
SELECT
    (n % 250) + 1,
    (ARRAY['pending', 'paid', 'shipped', 'cancelled'])[(n % 4) + 1],
    round((n * 7.31 % 500)::numeric, 2)
FROM generate_series(1, 500) AS n;

INSERT INTO analytics.events (name, payload)
SELECT 'event_' || n, jsonb_build_object('index', n)
FROM generate_series(1, 20) AS n;

CREATE TABLE public.places (
    id serial PRIMARY KEY,
    name text NOT NULL,
    location geometry(Point, 4326) NOT NULL
);

INSERT INTO public.places (name, location) VALUES
    ('Paris', ST_SetSRID(ST_MakePoint(2.3522, 48.8566), 4326)),
    ('Lyon', ST_SetSRID(ST_MakePoint(4.8357, 45.764), 4326)),
    ('Marseille', ST_SetSRID(ST_MakePoint(5.3698, 43.2965), 4326)),
    ('Bordeaux', ST_SetSRID(ST_MakePoint(-0.5792, 44.8378), 4326));

ANALYZE;
