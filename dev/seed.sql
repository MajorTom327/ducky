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

ANALYZE;
